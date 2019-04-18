// Copyright 2018 Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
//
// Portions Copyright 2017 The Chromium OS Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the THIRD-PARTY file.

//! Implements virtio devices, queues, and transport mechanisms.
extern crate epoll;
#[macro_use]
extern crate log;
extern crate pci;
extern crate virtio_bindings;
extern crate vm_memory;

use std::fmt;
use std::fs::File;
use std::io;

mod device;
mod queue;

pub use self::device::*;
pub use self::queue::*;

#[allow(dead_code)]
const DEVICE_INIT: u32 = 0x00;
#[allow(dead_code)]
const DEVICE_ACKNOWLEDGE: u32 = 0x01;
#[allow(dead_code)]
const DEVICE_DRIVER: u32 = 0x02;
#[allow(dead_code)]
const DEVICE_DRIVER_OK: u32 = 0x04;
#[allow(dead_code)]
const DEVICE_FEATURES_OK: u32 = 0x08;
#[allow(dead_code)]
const DEVICE_FAILED: u32 = 0x80;

#[allow(dead_code)]
const VIRTIO_F_VERSION_1: u32 = 32;

// Types taken from linux/virtio_ids.h
#[derive(Copy, Clone)]
#[allow(dead_code)]
#[allow(non_camel_case_types)]
#[repr(C)]
enum VirtioDeviceType {
    TYPE_NET = 1,
    TYPE_BLOCK = 2,
    TYPE_RNG = 4,
    TYPE_BALLOON = 5,
    TYPE_9P = 9,
    TYPE_GPU = 16,
    TYPE_INPUT = 18,
    TYPE_VSOCK = 19,
}

// In order to use the `{}` marker, the trait `fmt::Display` must be implemented
// manually for the type VirtioDeviceType.
impl fmt::Display for VirtioDeviceType {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let output = match *self {
            VirtioDeviceType::TYPE_NET => "net",
            VirtioDeviceType::TYPE_BLOCK => "block",
            VirtioDeviceType::TYPE_RNG => "rng",
            VirtioDeviceType::TYPE_BALLOON => "balloon",
            VirtioDeviceType::TYPE_GPU => "gpu",
            VirtioDeviceType::TYPE_9P => "9p",
            VirtioDeviceType::TYPE_VSOCK => "vsock",
            _ => return Err(std::fmt::Error),
        };
        write!(f, "{}", output)
    }
}

#[allow(dead_code)]
const INTERRUPT_STATUS_USED_RING: u32 = 0x1;
#[allow(dead_code)]
const INTERRUPT_STATUS_CONFIG_CHANGED: u32 = 0x2;

#[derive(Debug)]
pub enum ActivateError {
    EpollCtl(std::io::Error),
    BadActivate,
    #[cfg(feature = "vsock")]
    BadVhostActivate(self::vhost::Error),
}

pub type ActivateResult = std::result::Result<(), ActivateError>;

pub type DeviceEventT = u16;

/// The payload is used to handle events where the internal state of the VirtIO device
/// needs to be changed.
#[allow(clippy::large_enum_variant)]
pub enum EpollHandlerPayload {
    /// DrivePayload(disk_image)
    DrivePayload(File),
    /// Events that do not need a payload.
    Empty,
}

type Result<T> = std::result::Result<T, Error>;

pub trait EpollHandler: Send {
    fn handle_event(
        &mut self,
        device_event: DeviceEventT,
        event_flags: u32,
        payload: EpollHandlerPayload,
    ) -> Result<()>;
}

#[derive(Debug)]
pub enum Error {
    FailedReadingQueue {
        event_type: &'static str,
        underlying: io::Error,
    },
    FailedReadTap,
    FailedSignalingUsedQueue(io::Error),
    PayloadExpected,
    UnknownEvent {
        device: &'static str,
        event: DeviceEventT,
    },
    IoError(io::Error),
}