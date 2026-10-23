//! The I/O Driver chip (IOD): 256 I/O ports and a 256-entry FIFO interrupt queue.
//!
//! Port 0 is reserved for IOD <-> CPU communication. When a peripheral is
//! plugged in, the IOD sends two interrupts on port 0: first the peripheral's
//! first port number, then the peripheral's ID.

use std::collections::VecDeque;

pub const PORT_COUNT: usize = 256;
pub const QUEUE_CAPACITY: usize = 256;
pub const RESERVED_PORT: u8 = 0;

/// One queued interrupt: the port that raised it and the data that comes with it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Interrupt {
    pub port: u8,
    pub data: u8,
}

#[derive(Clone, Debug)]
pub struct Iod {
    /// Data latch of each port.
    ports: [u8; PORT_COUNT],
    /// Device ID owning each port (0 = free).
    owners: [u8; PORT_COUNT],
    queue: VecDeque<Interrupt>,
    /// Port written by the last `OUT`, waiting to be delivered to its peripheral.
    pending_write: Option<u8>,
}

impl Default for Iod {
    fn default() -> Self {
        Iod {
            ports: [0; PORT_COUNT],
            owners: [0; PORT_COUNT],
            queue: VecDeque::with_capacity(QUEUE_CAPACITY),
            pending_write: None,
        }
    }
}

impl Iod {
    /// Unplugs everything, clears ports and the queue.
    pub fn reset(&mut self) {
        self.ports = [0; PORT_COUNT];
        self.owners = [0; PORT_COUNT];
        self.queue.clear();
        self.pending_write = None;
    }

    /// Value currently latched on a port.
    #[inline(always)]
    pub fn port(&self, port: u8) -> u8 {
        self.ports[port as usize]
    }

    /// Sets a port latch from the peripheral side (no notification).
    #[inline(always)]
    pub fn set_port(&mut self, port: u8, value: u8) {
        self.ports[port as usize] = value;
    }

    /// `OUT` from the CPU: sets the latch and records the write so the
    /// owning peripheral can react to it.
    #[inline(always)]
    pub fn cpu_write(&mut self, port: u8, value: u8) {
        self.ports[port as usize] = value;
        self.pending_write = Some(port);
    }

    #[inline(always)]
    pub(crate) fn take_pending_write(&mut self) -> Option<u8> {
        self.pending_write.take()
    }

    /// Device ID owning a port (0 = free).
    pub fn owner(&self, port: u8) -> u8 {
        self.owners[port as usize]
    }

    pub fn ports(&self) -> &[u8; PORT_COUNT] {
        &self.ports
    }

    pub fn owners(&self) -> &[u8; PORT_COUNT] {
        &self.owners
    }

    pub fn queue(&self) -> impl ExactSizeIterator<Item = &Interrupt> {
        self.queue.iter()
    }

    #[inline(always)]
    pub fn has_pending_interrupt(&self) -> bool {
        !self.queue.is_empty()
    }

    pub(crate) fn pop_interrupt(&mut self) -> Option<Interrupt> {
        self.queue.pop_front()
    }

    /// Queues an interrupt on `port`, carrying the port's current data.
    /// Returns false (and drops the interrupt) if the queue is full.
    pub fn trigger_interrupt(&mut self, port: u8) -> bool {
        if self.queue.len() >= QUEUE_CAPACITY {
            return false;
        }
        self.queue.push_back(Interrupt {
            port,
            data: self.ports[port as usize],
        });
        true
    }

    /// Plugs a peripheral: allocates `count` consecutive ports starting at the
    /// first free port (port 0 excluded), then sends the two port-0 interrupts.
    /// Returns the first allocated port, or `None` if there is no room.
    pub fn connect(&mut self, device_id: u8, count: u8) -> Option<u8> {
        let first = (1..PORT_COUNT).find(|&p| self.owners[p] == 0)?;
        if count == 0 || PORT_COUNT - first < count as usize {
            return None;
        }
        for p in first..first + count as usize {
            self.owners[p] = device_id;
        }
        let first = first as u8;

        self.ports[RESERVED_PORT as usize] = first;
        self.trigger_interrupt(RESERVED_PORT);
        self.ports[RESERVED_PORT as usize] = device_id;
        self.trigger_interrupt(RESERVED_PORT);

        Some(first)
    }
}
