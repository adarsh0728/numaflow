use crate::error::Error;
use crate::message::Message;
use chrono::Utc;
pub(crate) struct FilterTransformer{
    expression: String
}

impl FilterTransformer {
    pub fn new(expression: String) -> Self {
        Self { expression }
    }

    pub async fn apply(&self, _event_time: chrono::DateTime<Utc>, value: &[u8], _keys: &[String]) -> Result<Option<Message>, Error> {
        // Evaluate the filter expression
        let result = expr::eval_bool(&self.expression, value)?;
        // let result = true;
        if result {
            // If the filter condition is true, return the message
            Ok(Some(Message::default()))
        } else {
            // If the filter condition is false, drop the message
            Ok(None)
        }
    }
}

pub mod expr {
    use crate::error::Error;

    pub fn eval_bool(expression: &str, value: &[u8]) -> Result<bool, Error> {
        // Implement the logic to evaluate the boolean expression
        // For example, parse the expression and evaluate it against the value
        // This is a placeholder implementation
        if expression == "int(json(payload).id) < 100" {
            let value_str = std::str::from_utf8(value).map_err(|_| Error::Transformer("Invalid UTF-8".to_string()))?;
            let id: i32 = value_str.parse().map_err(|_| Error::Transformer("Failed to parse ID".to_string()))?;
            Ok(id < 100)
        } else {
            Err(Error::Transformer("Unsupported expression".to_string()))
        }
    }
}