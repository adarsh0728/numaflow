use tokio_util::sync::CancellationToken;

use crate::error;
use crate::error::Error;
use crate::pipeline::isb::jetstream::writer::JetstreamWriter;
use crate::source::Source;

/// Source forwarder is the orchestrator which starts streaming source, a transformer, and an isb writer
/// and manages the lifecycle of these components.
pub(crate) struct SourceForwarder {
    source: Source,
    writer: JetstreamWriter,
}

impl SourceForwarder {
    pub(crate) fn new(source: Source, writer: JetstreamWriter) -> Self {
        Self { source, writer }
    }

    /// Start the forwarder by starting the streaming source, transformer, and writer.
    pub(crate) async fn start(self, cln_token: CancellationToken) -> error::Result<()> {
        let child_token = cln_token.child_token();
        let (messages_stream, reader_handle) = self.source.streaming_read(child_token.clone())?;

        let writer_handle = self
            .writer
            .streaming_write(messages_stream, child_token)
            .await?;

        let (reader_result, sink_writer_result) = tokio::try_join!(reader_handle, writer_handle)
            .map_err(|e| {
                error!(?e, "Error while joining reader and sink writer");
                Error::Forwarder(format!("Error while joining reader and sink writer: {e}"))
            })?;

        sink_writer_result.inspect_err(|e| {
            error!(?e, "Error while writing messages");
            cln_token.cancel();
        })?;

        reader_result.inspect_err(|e| {
            error!(?e, "Error while reading messages");
            cln_token.cancel();
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use async_nats::jetstream;
    use async_nats::jetstream::{consumer, stream};
    use chrono::Utc;
    use numaflow::source::{Message, Offset, SourceReadRequest};
    use numaflow::{source, sourcetransform};
    use numaflow_pb::clients::source::source_client::SourceClient;
    use numaflow_pb::clients::sourcetransformer::source_transform_client::SourceTransformClient;
    use tempfile::TempDir;
    use tokio::sync::mpsc::Sender;
    use tokio::sync::oneshot;
    use tokio::task::JoinHandle;
    use tokio_util::sync::CancellationToken;

    use crate::Result;
    use crate::config::pipeline::isb::{BufferWriterConfig, Stream};
    use crate::config::pipeline::{ToVertexConfig, VertexType};
    use crate::pipeline::forwarder::source_forwarder::SourceForwarder;
    use crate::pipeline::isb::jetstream::writer::JetstreamWriter;
    use crate::shared::grpc::create_rpc_channel;
    use crate::source::user_defined::new_source;
    use crate::source::{Source, SourceType};
    use crate::tracker::TrackerHandle;
    use crate::transformer::Transformer;

    struct SimpleSource {
        num: usize,
        sent_count: AtomicUsize,
        yet_to_ack: std::sync::RwLock<HashSet<String>>,
    }

    impl SimpleSource {
        fn new(num: usize) -> Self {
            Self {
                num,
                sent_count: AtomicUsize::new(0),
                yet_to_ack: std::sync::RwLock::new(HashSet::new()),
            }
        }
    }

    #[tonic::async_trait]
    impl source::Sourcer for SimpleSource {
        async fn read(&self, request: SourceReadRequest, transmitter: Sender<Message>) {
            let event_time = Utc::now();
            let mut message_offsets = Vec::with_capacity(request.count);

            for i in 0..request.count {
                if self.sent_count.load(Ordering::SeqCst) >= self.num {
                    return;
                }

                let offset = format!("{}-{}", event_time.timestamp_nanos_opt().unwrap(), i);
                transmitter
                    .send(Message {
                        value: b"hello".to_vec(),
                        event_time,
                        offset: Offset {
                            offset: offset.clone().into_bytes(),
                            partition_id: 0,
                        },
                        keys: vec![],
                        headers: Default::default(),
                    })
                    .await
                    .unwrap();
                message_offsets.push(offset);
                self.sent_count.fetch_add(1, Ordering::SeqCst);
            }
            self.yet_to_ack.write().unwrap().extend(message_offsets);
        }

        async fn ack(&self, offsets: Vec<Offset>) {
            for offset in offsets {
                self.yet_to_ack
                    .write()
                    .unwrap()
                    .remove(&String::from_utf8(offset.offset).unwrap());
            }
        }

        async fn pending(&self) -> Option<usize> {
            Some(
                self.num - self.sent_count.load(Ordering::SeqCst)
                    + self.yet_to_ack.read().unwrap().len(),
            )
        }

        async fn partitions(&self) -> Option<Vec<i32>> {
            Some(vec![1, 2])
        }
    }

    struct SimpleTransformer;

    #[tonic::async_trait]
    impl sourcetransform::SourceTransformer for SimpleTransformer {
        async fn transform(
            &self,
            input: sourcetransform::SourceTransformRequest,
        ) -> Vec<sourcetransform::Message> {
            let message =
                sourcetransform::Message::new(input.value, Utc::now()).with_keys(input.keys);
            vec![message]
        }
    }

    #[cfg(feature = "nats-tests")]
    #[tokio::test]
    async fn test_source_forwarder() {
        let tracker_handle = TrackerHandle::new(None);

        // create the source which produces x number of messages
        let cln_token = CancellationToken::new();

        // create a transformer
        let (st_shutdown_tx, st_shutdown_rx) = oneshot::channel();
        let tmp_dir = TempDir::new().unwrap();
        let sock_file = tmp_dir.path().join("sourcetransform.sock");
        let server_info_file = tmp_dir.path().join("sourcetransformer-server-info");

        let server_info = server_info_file.clone();
        let server_socket = sock_file.clone();
        let transformer_handle = tokio::spawn(async move {
            sourcetransform::Server::new(SimpleTransformer)
                .with_socket_file(server_socket)
                .with_server_info_file(server_info)
                .start_with_shutdown(st_shutdown_rx)
                .await
                .expect("server failed");
        });

        // wait for the server to start
        tokio::time::sleep(Duration::from_millis(100)).await;
        let client = SourceTransformClient::new(create_rpc_channel(sock_file).await.unwrap());
        let transformer = Transformer::new(
            10,
            10,
            Duration::from_secs(10),
            client,
            tracker_handle.clone(),
        )
        .await
        .unwrap();

        let (src_shutdown_tx, src_shutdown_rx) = oneshot::channel();
        let tmp_dir = TempDir::new().unwrap();
        let sock_file = tmp_dir.path().join("source.sock");
        let server_info_file = tmp_dir.path().join("source-server-info");

        let server_info = server_info_file.clone();
        let server_socket = sock_file.clone();
        let source_handle = tokio::spawn(async move {
            // a simple source which generates total of 100 messages
            source::Server::new(SimpleSource::new(100))
                .with_socket_file(server_socket)
                .with_server_info_file(server_info)
                .start_with_shutdown(src_shutdown_rx)
                .await
                .unwrap()
        });

        // wait for the server to start
        // TODO: flaky
        tokio::time::sleep(Duration::from_millis(100)).await;

        let client = SourceClient::new(create_rpc_channel(sock_file).await.unwrap());

        let (src_read, src_ack, lag_reader) = new_source(client, 5, Duration::from_millis(1000))
            .await
            .map_err(|e| panic!("failed to create source reader: {:?}", e))
            .unwrap();

        let source = Source::new(
            5,
            SourceType::UserDefinedSource(Box::new(src_read), Box::new(src_ack), lag_reader),
            tracker_handle.clone(),
            true,
            Some(transformer),
            None,
        );

        // create a js writer
        let js_url = "localhost:4222";
        // Create JetStream context
        let client = async_nats::connect(js_url).await.unwrap();
        let context = jetstream::new(client);

        let stream = Stream::new("test_source_forwarder", "test", 0);
        // Delete stream if it exists
        let _ = context.delete_stream(stream.name).await;
        let _stream = context
            .get_or_create_stream(stream::Config {
                name: stream.name.to_string(),
                subjects: vec![stream.name.into()],
                max_message_size: 1024,
                ..Default::default()
            })
            .await
            .unwrap();

        let _consumer = context
            .create_consumer_on_stream(
                consumer::Config {
                    name: Some(stream.name.to_string()),
                    ack_policy: consumer::AckPolicy::Explicit,
                    ..Default::default()
                },
                stream.name,
            )
            .await
            .unwrap();

        use crate::pipeline::isb::jetstream::writer::ISBWriterConfig;
        let writer = JetstreamWriter::new(ISBWriterConfig {
            config: vec![ToVertexConfig {
                partitions: 1,
                writer_config: BufferWriterConfig {
                    streams: vec![stream.clone()],
                    ..Default::default()
                },
                conditions: None,
                name: "test-vertex",
                to_vertex_type: VertexType::MapUDF,
            }],
            js_ctx: context.clone(),
            paf_concurrency: 100,
            tracker_handle: tracker_handle.clone(),
            cancel_token: cln_token.clone(),
            watermark_handle: None,
            vertex_type: VertexType::Source,
            isb_config: None,
        });

        // create the forwarder with the source, transformer, and writer
        let forwarder = SourceForwarder::new(source.clone(), writer);

        let cancel_token = cln_token.clone();
        let forwarder_handle: JoinHandle<Result<()>> = tokio::spawn(async move {
            forwarder.start(cancel_token).await?;
            Ok(())
        });

        // wait for one sec to check if the pending becomes zero, because all the messages
        // should be read and acked; if it doesn't, then fail the test
        let tokio_result = tokio::time::timeout(Duration::from_secs(1), async move {
            loop {
                let pending = source.pending().await.unwrap();
                if pending == Some(0) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await;

        assert!(
            tokio_result.is_ok(),
            "Timeout occurred before pending became zero"
        );
        cln_token.cancel();
        forwarder_handle.await.unwrap().unwrap();
        st_shutdown_tx.send(()).unwrap();
        src_shutdown_tx.send(()).unwrap();
        source_handle.await.unwrap();
        transformer_handle.await.unwrap();
    }
}
