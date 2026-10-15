use std::ops::{Deref, DerefMut};

use crate::{trace::TraceContext, Interface};

pub struct Raw {
    interface: Box<dyn Interface>,
    pub(crate) trace: Option<TraceContext>,
}

impl Raw {
    pub fn new(interface: impl Interface + 'static) -> Self {
        Self {
            interface: Box::new(interface),
            trace: None,
        }
    }

    pub(crate) fn new_with_trace(
        interface: impl Interface + 'static,
        trace: Option<TraceContext>,
    ) -> Self {
        Self {
            interface: Box::new(interface),
            trace,
        }
    }

    #[must_use]
    pub fn trace_context(&self) -> Option<&TraceContext> {
        self.trace.as_ref()
    }
}

impl Deref for Raw {
    type Target = Box<dyn Interface>;

    fn deref(&self) -> &Self::Target {
        &self.interface
    }
}

impl DerefMut for Raw {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.interface
    }
}
