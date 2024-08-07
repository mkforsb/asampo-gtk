// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use std::collections::VecDeque;

use crate::{appmessage::AppMessage, model::AnyhowResult};

#[derive(Debug, Clone, Default)]
pub struct MessageQueue {
    messages: VecDeque<AppMessage>,
}

impl MessageQueue {
    pub fn new() -> MessageQueue {
        MessageQueue::default()
    }

    pub fn enqueue_message(self, message: AppMessage) -> MessageQueue {
        let mut result = self.clone();
        result.messages.push_back(message);
        result
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    pub fn pop_message(self) -> AnyhowResult<(MessageQueue, AppMessage)> {
        let mut result = self.clone();
        let message = result
            .messages
            .pop_front()
            .ok_or(anyhow::anyhow!("No message available"))?;
        Ok((result, message))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_queue() {
        let queue = MessageQueue::new();

        assert_eq!(queue.len(), 0);
        assert!(queue.is_empty());
        assert!(queue.clone().pop_message().is_err());

        let queue = queue.enqueue_message(AppMessage::TimerTick);

        assert_eq!(queue.len(), 1);
        assert!(!queue.is_empty());
        assert!(queue.clone().pop_message().is_ok());
        assert!(matches!(
            queue.pop_message().unwrap().1,
            AppMessage::TimerTick
        ));
    }
}
