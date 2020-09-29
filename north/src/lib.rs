// Copyright (c) 2019 - 2020 ESRLabs
//
//   Licensed under the Apache License, Version 2.0 (the "License");
//   you may not use this file except in compliance with the License.
//   You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
//   Unless required by applicable law or agreed to in writing, software
//   distributed under the License is distributed on an "AS IS" BASIS,
//   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//   See the License for the specific language governing permissions and
//   limitations under the License.

#![deny(clippy::all)]

#[cfg(any(target_os = "android", target_os = "linux"))]
#[macro_use]
extern crate structure;

#[cfg(feature = "api")]
pub mod api;
#[cfg(feature = "manifest")]
pub mod manifest;
#[cfg(feature = "runtime")]
pub mod runtime;

// TODO: Move to config
pub const SYSTEM_UID: u32 = 1000;
// TODO: Move to config
pub const SYSTEM_GID: u32 = 1000;