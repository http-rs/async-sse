/// An SSE message.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Message {
    /// The ID of this event.
    ///
    /// See also the [Server-Sent Events spec](https://html.spec.whatwg.org/multipage/server-sent-events.html#concept-event-stream-last-event-id).
    pub(crate) id: Option<String>,
    /// The event type. Defaults to "message" if no event name is provided.
    pub(crate) event: String,
    /// The data for this event.
    pub(crate) data: Vec<u8>,
}

impl Message {
    /// Get the message id.
    pub fn id(&self) -> &Option<String> {
        &self.id
    }

    /// Get the message event name.
    pub fn name(&self) -> &String {
        &self.event
    }

    /// Access the event data.
    pub fn data(&self) -> &[u8] {
        &self.data
    }
}
