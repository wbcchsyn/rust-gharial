// Copyright 2020 Shin Yoshida
//
// "LGPL-3.0-or-later OR Apache-2.0 OR BSD-2-Clause OR MIT"
//
// This is part of test-allocator
//
//  test-allocator is free software: you can redistribute it and/or modify
//  it under the terms of the GNU Lesser General Public License as published by
//  the Free Software Foundation, either version 3 of the License, or
//  (at your option) any later version.
//
//  test-allocator is distributed in the hope that it will be useful,
//  but WITHOUT ANY WARRANTY; without even the implied warranty of
//  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
//  GNU Lesser General Public License for more details.
//
//  You should have received a copy of the GNU Lesser General Public License
//  along with test-allocator.  If not, see <http://www.gnu.org/licenses/>.
//
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
//
// Redistribution and use in source and binary forms, with or without modification, are permitted
// provided that the following conditions are met:
//
// 1. Redistributions of source code must retain the above copyright notice, this list of
//    conditions and the following disclaimer.
// 2. Redistributions in binary form must reproduce the above copyright notice, this
//    list of conditions and the following disclaimer in the documentation and/or other
//    materials provided with the distribution.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND
// ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
// WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
// IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT,
// INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT
// NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
// PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
// WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
// ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
// POSSIBILITY OF SUCH DAMAGE.
//
//
// Permission is hereby granted, free of charge, to any person obtaining a copy of this software
// and associated documentation files (the "Software"), to deal in the Software without
// restriction, including without limitation the rights to use, copy, modify, merge, publish,
// distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the
// Software is furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice (including the next paragraph) shall be
// included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING
// BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
// NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
// DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

//! # alloc

use core::alloc::{GlobalAlloc, Layout};
use std::alloc::System;
use std::collections::hash_map::HashMap;
use std::sync::{Arc, Mutex};

/// `TestAlloc` is a wrapper of `GlobalAlloc` for test.
/// This checks the followings.
///
/// - Method `dealloc` checks the argument `layout` matches to what passed to `alloc` .
/// - All the allocated memories have already been deallocated on the drop.
///   (Cloned instances shares the allocating memory information. It is checked
///   when the last cloned instance is dropped.)
pub struct TestAlloc<A = System>
where
    A: GlobalAlloc,
{
    alloc: A,
    allocatings: Arc<Mutex<HashMap<*mut u8, Layout>>>,
}

impl<A> Default for TestAlloc<A>
where
    A: GlobalAlloc + Default,
{
    fn default() -> Self {
        Self::from(A::default())
    }
}

impl<A> From<A> for TestAlloc<A>
where
    A: GlobalAlloc,
{
    fn from(inner: A) -> Self {
        Self {
            alloc: inner,
            allocatings: Arc::default(),
        }
    }
}

impl<A> Clone for TestAlloc<A>
where
    A: GlobalAlloc + Clone,
{
    fn clone(&self) -> Self {
        Self {
            alloc: self.alloc.clone(),
            allocatings: self.allocatings.clone(),
        }
    }
}

impl<A> Drop for TestAlloc<A>
where
    A: GlobalAlloc,
{
    fn drop(&mut self) {
        if Arc::strong_count(&self.allocatings) == 1 {
            let allocatings = self.allocatings.lock().unwrap();
            assert_eq!(true, allocatings.is_empty());
        }
    }
}

unsafe impl<A> GlobalAlloc for TestAlloc<A>
where
    A: GlobalAlloc,
{
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = self.alloc.alloc(layout);
        if !ptr.is_null() {
            let mut allocatings = self.allocatings.lock().unwrap();
            let prev = allocatings.insert(ptr, layout);
            assert_eq!(true, prev.is_none());
        }

        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // `GlobalAlloc::dealloc` interface does not define the behavior when ptr is null.
        assert_eq!(false, ptr.is_null());

        // Enclose to release the lock as soon as possible.
        {
            let mut allocatings = self.allocatings.lock().unwrap();
            let prev = allocatings.remove(&ptr).unwrap();
            assert_eq!(layout, prev);
        }

        self.alloc.dealloc(ptr, layout);
    }
}

/// `FailureAlloc` is an implementation for `GlobalAlloc` .
///
/// It always fails to allocate memory. i.e. `FailureAlloc::alloc` always
/// returns null pointer.
#[derive(Clone, Copy)]
pub struct FailureAlloc;

impl Default for FailureAlloc {
    fn default() -> Self {
        Self
    }
}

unsafe impl GlobalAlloc for FailureAlloc {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        core::ptr::null_mut()
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        assert!(false)
    }
}

/// `RandomFailureAlloc<A>` is a wrapper of `GlobalAlloc` .
///
/// It usually delegates the requests to the backend allocator `A` , however,
/// the allocation will fail occasionally. i.e. `RandomFailureAlloc::alloc` can
/// return null pointer before having consumed the OS memory.
///
/// The failure properbility is 1/16.
pub struct RandomFailureAlloc<A = TestAlloc<System>>
where
    A: GlobalAlloc,
{
    alloc: A,
}

impl<A> Default for RandomFailureAlloc<A>
where
    A: GlobalAlloc + Default,
{
    fn default() -> Self {
        Self::from(A::default())
    }
}

impl<A> From<A> for RandomFailureAlloc<A>
where
    A: GlobalAlloc,
{
    fn from(alloc: A) -> Self {
        Self { alloc }
    }
}

impl<A> Clone for RandomFailureAlloc<A>
where
    A: GlobalAlloc + Clone,
{
    fn clone(&self) -> Self {
        Self::from(self.alloc.clone())
    }
}
