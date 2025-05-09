/*
Copyright 2022 The Numaproj Authors.

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
*/

package udsource

import (
	"context"
	"errors"
	"fmt"
	"testing"
	"time"

	sourcepb "github.com/numaproj/numaflow-go/pkg/apis/proto/source/v1"
	"github.com/numaproj/numaflow-go/pkg/apis/proto/source/v1/sourcemock"
	"github.com/stretchr/testify/assert"
	"go.uber.org/goleak"
	"go.uber.org/mock/gomock"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
	"google.golang.org/protobuf/types/known/timestamppb"

	"github.com/numaproj/numaflow/pkg/isb"
	sourceclient "github.com/numaproj/numaflow/pkg/sdkclient/source"
)

func TestMain(m *testing.M) {
	goleak.VerifyTestMain(m)
}

func NewMockUDSgRPCBasedUDSource(ctx context.Context, mockClient *sourcemock.MockSourceClient) *GRPCBasedUDSource {
	c, _ := sourceclient.NewFromClient(ctx, mockClient)
	return &GRPCBasedUDSource{
		vertexName:         "testVertex",
		pipelineName:       "testPipeline",
		vertexReplicaIndex: 0,
		client:             c,
	}
}

func Test_gRPCBasedUDSource_WaitUntilReadyWithMockClient(t *testing.T) {
	ctrl := gomock.NewController(t)
	defer ctrl.Finish()

	mockClient := sourcemock.NewMockSourceClient(ctrl)
	mockClient.EXPECT().IsReady(gomock.Any(), gomock.Any()).Return(&sourcepb.ReadyResponse{Ready: true}, nil)
	mockClient.EXPECT().ReadFn(gomock.Any(), gomock.Any()).Return(nil, nil)
	mockClient.EXPECT().AckFn(gomock.Any(), gomock.Any()).Return(nil, nil)

	ctx, cancel := context.WithTimeout(context.Background(), time.Second)
	defer cancel()
	go func() {
		<-ctx.Done()
		if errors.Is(ctx.Err(), context.DeadlineExceeded) {
			t.Log(t.Name(), "test timeout")
		}
	}()

	u := NewMockUDSgRPCBasedUDSource(ctx, mockClient)
	err := u.WaitUntilReady(ctx)
	assert.NoError(t, err)
}

func Test_gRPCBasedUDSource_ApplyPendingWithMockClient(t *testing.T) {
	t.Run("test success - positive pending count", func(t *testing.T) {
		ctrl := gomock.NewController(t)
		defer ctrl.Finish()

		testResponse := &sourcepb.PendingResponse{
			Result: &sourcepb.PendingResponse_Result{
				Count: 123,
			},
		}

		mockSourceClient := sourcemock.NewMockSourceClient(ctrl)
		mockSourceClient.EXPECT().PendingFn(gomock.Any(), gomock.Any()).Return(testResponse, nil).AnyTimes()
		mockSourceClient.EXPECT().ReadFn(gomock.Any(), gomock.Any()).Return(nil, nil)
		mockSourceClient.EXPECT().AckFn(gomock.Any(), gomock.Any()).Return(nil, nil)

		ctx, cancel := context.WithTimeout(context.Background(), time.Second)
		defer cancel()
		go func() {
			<-ctx.Done()
			if errors.Is(ctx.Err(), context.DeadlineExceeded) {
				t.Log(t.Name(), "test timeout")
			}
		}()

		u := NewMockUDSgRPCBasedUDSource(ctx, mockSourceClient)
		count, err := u.ApplyPendingFn(ctx)
		assert.NoError(t, err)
		assert.Equal(t, int64(123), count)
	})

	t.Run("test success - pending is not available - negative count", func(t *testing.T) {
		ctrl := gomock.NewController(t)
		defer ctrl.Finish()

		testResponse := &sourcepb.PendingResponse{
			Result: &sourcepb.PendingResponse_Result{
				Count: -123,
			},
		}

		mockSourceClient := sourcemock.NewMockSourceClient(ctrl)
		mockSourceClient.EXPECT().PendingFn(gomock.Any(), gomock.Any()).Return(testResponse, nil).AnyTimes()
		mockSourceClient.EXPECT().ReadFn(gomock.Any(), gomock.Any()).Return(nil, nil)
		mockSourceClient.EXPECT().AckFn(gomock.Any(), gomock.Any()).Return(nil, nil)

		ctx, cancel := context.WithTimeout(context.Background(), time.Second)
		defer cancel()
		go func() {
			<-ctx.Done()
			if errors.Is(ctx.Err(), context.DeadlineExceeded) {
				t.Log(t.Name(), "test timeout")
			}
		}()

		u := NewMockUDSgRPCBasedUDSource(ctx, mockSourceClient)
		count, err := u.ApplyPendingFn(ctx)
		assert.NoError(t, err)
		assert.Equal(t, isb.PendingNotAvailable, count)
	})

	t.Run("test err", func(t *testing.T) {
		ctrl := gomock.NewController(t)
		defer ctrl.Finish()

		testResponse := &sourcepb.PendingResponse{
			Result: &sourcepb.PendingResponse_Result{
				Count: 123,
			},
		}

		mockSourceErrClient := sourcemock.NewMockSourceClient(ctrl)
		mockSourceErrClient.EXPECT().PendingFn(gomock.Any(), gomock.Any()).Return(testResponse, fmt.Errorf("mock udsource pending error")).AnyTimes()
		mockSourceErrClient.EXPECT().ReadFn(gomock.Any(), gomock.Any()).Return(nil, nil)
		mockSourceErrClient.EXPECT().AckFn(gomock.Any(), gomock.Any()).Return(nil, nil)

		ctx, cancel := context.WithTimeout(context.Background(), time.Second)
		defer cancel()
		go func() {
			<-ctx.Done()
			if errors.Is(ctx.Err(), context.DeadlineExceeded) {
				t.Log(t.Name(), "test timeout")
			}
		}()

		u := NewMockUDSgRPCBasedUDSource(ctx, mockSourceErrClient)
		count, err := u.ApplyPendingFn(ctx)

		assert.Equal(t, isb.PendingNotAvailable, count)
		assert.Equal(t, fmt.Errorf("mock udsource pending error"), err)
	})
}

func Test_gRPCBasedUDSource_ApplyReadWithMockClient(t *testing.T) {
	t.Run("test success", func(t *testing.T) {
		ctrl := gomock.NewController(t)
		defer ctrl.Finish()
		mockClient := sourcemock.NewMockSourceClient(ctrl)
		mockReadClient := sourcemock.NewMockSource_ReadFnClient(ctrl)
		mockClient.EXPECT().ReadFn(gomock.Any(), gomock.Any()).Return(mockReadClient, nil)
		mockClient.EXPECT().AckFn(gomock.Any(), gomock.Any()).Return(nil, nil)

		offset := &sourcepb.Offset{Offset: []byte(`test_offset`), PartitionId: 0}

		var TestEventTime = time.Unix(1661169600, 0).UTC()
		expectedResponse := &sourcepb.ReadResponse{
			Result: &sourcepb.ReadResponse_Result{
				Payload:   []byte(`test_payload`),
				Offset:    offset,
				EventTime: timestamppb.New(TestEventTime),
				Keys:      []string{"test_key"},
			},
		}
		mockReadClient.EXPECT().Recv().Return(expectedResponse, nil).Times(1)

		eotResponse := &sourcepb.ReadResponse{Status: &sourcepb.ReadResponse_Status{Eot: true}}
		mockReadClient.EXPECT().Recv().Return(eotResponse, nil).Times(1)

		req := &sourcepb.ReadRequest{
			Request: &sourcepb.ReadRequest_Request{
				NumRecords:  1,
				TimeoutInMs: 1000,
			},
		}
		mockReadClient.EXPECT().Send(req).Return(nil).Times(1)

		ctx, cancel := context.WithTimeout(context.Background(), time.Second)
		defer cancel()
		go func() {
			<-ctx.Done()
			if errors.Is(ctx.Err(), context.DeadlineExceeded) {
				t.Log(t.Name(), "test timeout")
			}
		}()

		u := NewMockUDSgRPCBasedUDSource(ctx, mockClient)
		readMessages, err := u.ApplyReadFn(ctx, 1, time.Millisecond*1000)
		assert.NoError(t, err)
		assert.Equal(t, 1, len(readMessages))
		assert.Equal(t, []byte(`test_payload`), readMessages[0].Body.Payload)
		assert.Equal(t, []string{"test_key"}, readMessages[0].Keys)
		assert.Equal(t, NewUserDefinedSourceOffset(offset), readMessages[0].ReadOffset)
		assert.Equal(t, TestEventTime, readMessages[0].EventTime)
	})

	t.Run("test error", func(t *testing.T) {
		ctrl := gomock.NewController(t)
		defer ctrl.Finish()

		mockClient := sourcemock.NewMockSourceClient(ctrl)
		mockReadClient := sourcemock.NewMockSource_ReadFnClient(ctrl)
		mockClient.EXPECT().ReadFn(gomock.Any(), gomock.Any()).Return(mockReadClient, nil)
		mockClient.EXPECT().AckFn(gomock.Any(), gomock.Any()).Return(nil, nil)

		req := &sourcepb.ReadRequest{
			Request: &sourcepb.ReadRequest_Request{
				NumRecords:  1,
				TimeoutInMs: 1000,
			},
		}
		mockReadClient.EXPECT().Send(req).Return(nil).Times(1)

		var TestEventTime = time.Unix(1661169600, 0).UTC()
		expectedResponse := &sourcepb.ReadResponse{
			Result: &sourcepb.ReadResponse_Result{
				Payload:   []byte(`test_payload`),
				Offset:    &sourcepb.Offset{Offset: []byte(`test_offset`), PartitionId: 0},
				EventTime: timestamppb.New(TestEventTime),
				Keys:      []string{"test_key"},
			},
		}
		mockReadClient.EXPECT().Recv().Return(expectedResponse, errors.New("mock error for read")).AnyTimes()
		ctx, cancel := context.WithTimeout(context.Background(), time.Second)
		defer cancel()
		go func() {
			<-ctx.Done()
			if errors.Is(ctx.Err(), context.DeadlineExceeded) {
				t.Log(t.Name(), "test timeout")
			}
		}()

		u := NewMockUDSgRPCBasedUDSource(ctx, mockClient)
		readMessages, err := u.ApplyReadFn(ctx, 1, time.Millisecond*1000)
		assert.Error(t, err)
		assert.Equal(t, 0, len(readMessages))
	})
}

func Test_gRPCBasedUDSource_ApplyAckWithMockClient(t *testing.T) {
	t.Run("test success", func(t *testing.T) {
		ctrl := gomock.NewController(t)
		defer ctrl.Finish()

		offset1 := &sourcepb.Offset{Offset: []byte("test-offset-1"), PartitionId: 0}
		offset2 := &sourcepb.Offset{Offset: []byte("test-offset-2"), PartitionId: 0}

		mockClient := sourcemock.NewMockSourceClient(ctrl)
		mockAckClient := sourcemock.NewMockSource_AckFnClient(ctrl)
		mockClient.EXPECT().ReadFn(gomock.Any(), gomock.Any()).Return(nil, nil)
		mockClient.EXPECT().AckFn(gomock.Any(), gomock.Any()).Return(mockAckClient, nil)

		req := &sourcepb.AckRequest{
			Request: &sourcepb.AckRequest_Request{
				Offsets: []*sourcepb.Offset{offset1, offset2},
			},
		}

		mockAckClient.EXPECT().Send(req).Return(nil).Times(1)
		mockAckClient.EXPECT().Recv().Return(&sourcepb.AckResponse{}, nil).Times(1)

		ctx, cancel := context.WithTimeout(context.Background(), time.Second)
		defer cancel()
		go func() {
			<-ctx.Done()
			if errors.Is(ctx.Err(), context.DeadlineExceeded) {
				t.Log(t.Name(), "test timeout")
			}
		}()

		u := NewMockUDSgRPCBasedUDSource(ctx, mockClient)
		err := u.ApplyAckFn(ctx, []isb.Offset{
			NewUserDefinedSourceOffset(offset1),
			NewUserDefinedSourceOffset(offset2),
		})
		assert.NoError(t, err)
	})

	t.Run("test error", func(t *testing.T) {
		ctrl := gomock.NewController(t)
		defer ctrl.Finish()

		offset1 := &sourcepb.Offset{Offset: []byte("test-offset-1"), PartitionId: 0}
		offset2 := &sourcepb.Offset{Offset: []byte("test-offset-2"), PartitionId: 0}

		mockClient := sourcemock.NewMockSourceClient(ctrl)
		mockAckClient := sourcemock.NewMockSource_AckFnClient(ctrl)
		mockClient.EXPECT().ReadFn(gomock.Any(), gomock.Any()).Return(nil, nil)
		mockClient.EXPECT().AckFn(gomock.Any(), gomock.Any()).Return(mockAckClient, nil)

		req1 := &sourcepb.AckRequest{
			Request: &sourcepb.AckRequest_Request{
				Offsets: []*sourcepb.Offset{offset1, offset2},
			},
		}

		mockAckClient.EXPECT().Send(req1).Return(status.New(codes.DeadlineExceeded, "mock test err").Err()).Times(1)

		ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
		defer cancel()
		go func() {
			<-ctx.Done()
			if errors.Is(ctx.Err(), context.DeadlineExceeded) {
				t.Log(t.Name(), "test timeout")
			}
		}()

		u := NewMockUDSgRPCBasedUDSource(ctx, mockClient)
		err := u.ApplyAckFn(ctx, []isb.Offset{
			NewUserDefinedSourceOffset(offset1),
			NewUserDefinedSourceOffset(offset2),
		})
		assert.Equal(t, err.Error(), fmt.Sprintf("failed to send ack request: %s", status.New(codes.DeadlineExceeded, "mock test err").Err()))
	})
}
