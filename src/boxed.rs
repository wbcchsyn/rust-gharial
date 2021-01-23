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

use crate::TestAlloc;
use core::alloc::{GlobalAlloc, Layout};
use core::cmp::Ordering;
use core::ops::{Deref, DerefMut};
use std::alloc::handle_alloc_error;
use std::borrow::{Borrow, BorrowMut};
use std::hash::{Hash, Hasher};

/// `TestBox` behaves like `std::boxed::Box` except for it owns a reference to a `GlobalAlloc` .
///
/// The default type of the `GlobalAlloc` reference is &[`TestAlloc`] .
/// Unlike to `std::boxed::Box` , it cause an assertion error when the `GlobalAlloc` is dropped
/// unless `TestBox` is surely dropped.
///
/// For example, it sometimes requires to allocate heap memory to implement container struct,
/// and then the elements must be dropped manually. `TestBox` helps the test to make sure the elements
/// are dropped.
///
/// [`TestAlloc`]: struct.TestAlloc.html
#[derive(Debug)]
pub struct TestBox<T, A = TestAlloc>
where
    A: GlobalAlloc,
{
    ptr: *mut T,
    alloc: A,
}

impl<T, A> Default for TestBox<T, A>
where
    T: Default,
    A: Default + GlobalAlloc,
{
    fn default() -> Self {
        Self::new(T::default(), A::default())
    }
}

impl<T, A> From<T> for TestBox<T, A>
where
    A: Default + GlobalAlloc,
{
    fn from(val: T) -> Self {
        Self::new(val, A::default())
    }
}

impl<T, A> TestBox<T, A>
where
    A: GlobalAlloc,
{
    /// Creates a new instance.
    ///
    /// # Examples
    ///
    /// ```
    /// use gharial::{TestAlloc, TestBox};
    /// use std::alloc::System;
    ///
    /// let alloc = TestAlloc::from(System);
    /// let _box = TestBox::new(5, alloc);
    /// ```
    pub fn new(x: T, alloc: A) -> Self {
        let layout = Layout::new::<T>();
        let ptr = unsafe { alloc.alloc(layout) as *mut T };
        if ptr.is_null() {
            handle_alloc_error(layout);
        }

        unsafe { ptr.write(x) };
        Self { ptr, alloc }
    }

    /// Creates a new instance from raw pointer and a reference to allocator.
    ///
    /// After calling this function, the raw pointer is owned by the resulting `TestBox` .
    /// Specifically, `TestBox::drop` destructs the referenced object and free the pointer.
    ///
    /// # Safety
    ///
    /// To use this function safe, the ptr should be allocated via `alloc` and it should not be
    /// freed anywhere else.
    ///
    /// # Examples
    ///
    /// ```
    /// use gharial::{TestAlloc, TestBox};
    /// use std::alloc::{handle_alloc_error, GlobalAlloc, Layout, System};
    ///
    /// let alloc = TestAlloc::from(System);
    /// let ptr = unsafe {
    ///     let layout = Layout::new::<i32>();
    ///     let ptr = alloc.alloc(layout) as *mut i32;
    ///     if ptr.is_null() {
    ///         handle_alloc_error(layout);
    ///     }
    ///
    ///     *ptr = 5;
    ///     ptr
    /// };
    ///
    /// let _box = unsafe { TestBox::from_raw_alloc(ptr, alloc) };
    /// ```
    pub unsafe fn from_raw_alloc(ptr: *mut T, alloc: A) -> Self {
        Self { ptr, alloc }
    }
}

impl<T, A> Clone for TestBox<T, A>
where
    T: Clone,
    A: Clone + GlobalAlloc,
{
    fn clone(&self) -> Self {
        Self::new(self.as_ref().clone(), self.alloc.clone())
    }
}

impl<T, A> Drop for TestBox<T, A>
where
    A: GlobalAlloc,
{
    fn drop(&mut self) {
        if self.ptr.is_null() {
            return;
        }

        unsafe {
            self.ptr.drop_in_place();
            let layout = Layout::new::<T>();
            self.alloc.dealloc(self.ptr as *mut u8, layout);
        }
    }
}

impl<T, A> PartialEq<Self> for TestBox<T, A>
where
    T: PartialEq,
    A: GlobalAlloc,
{
    fn eq(&self, rh: &Self) -> bool {
        let l: &T = self.borrow();
        let r: &T = rh.borrow();
        l == r
    }
}

impl<T, A> Eq for TestBox<T, A>
where
    T: Eq,
    A: GlobalAlloc,
{
}

impl<T, A> PartialOrd<Self> for TestBox<T, A>
where
    T: PartialOrd,
    A: GlobalAlloc,
{
    fn partial_cmp(&self, rh: &Self) -> Option<Ordering> {
        let l: &T = self.borrow();
        let r: &T = rh.borrow();
        l.partial_cmp(r)
    }
}

impl<T, A> Ord for TestBox<T, A>
where
    T: Ord,
    A: GlobalAlloc,
{
    fn cmp(&self, rh: &Self) -> Ordering {
        let l: &T = self.borrow();
        let r: &T = rh.borrow();
        l.cmp(r)
    }
}

impl<T, A> Hash for TestBox<T, A>
where
    T: Hash,
    A: GlobalAlloc,
{
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        let t: &T = self.borrow();
        t.hash(state)
    }
}

impl<T, A> TestBox<T, A>
where
    A: GlobalAlloc,
{
    /// Consumes and leaks `TestBox` .
    ///
    /// # Examples
    ///
    /// ```
    /// use gharial::{TestAlloc, TestBox};
    ///
    /// let alloc = TestAlloc::default();
    ///
    /// let five: TestBox<i32> = TestBox::new(5, alloc.clone());
    /// let leaked = TestBox::leak(five);
    /// assert_eq!(5, *leaked);
    ///
    /// let five_ = unsafe { TestBox::from_raw_alloc(leaked, alloc) };
    /// ```
    pub fn leak<'a>(mut tb: Self) -> &'a mut T
    where
        T: 'a,
    {
        let ptr = tb.ptr;
        tb.ptr = core::ptr::null_mut();

        unsafe { &mut *ptr }
    }

    /// Consumes the `TestBox` and returning a wrapped raw pointer.
    ///
    /// # Examples
    ///
    /// ```
    /// use gharial::{TestAlloc, TestBox};
    ///
    /// let alloc = TestAlloc::default();
    ///
    /// let five: TestBox<i32> = TestBox::new(5, alloc.clone());
    /// let raw = TestBox::into_raw(five);
    /// assert_eq!(5, unsafe { *raw });
    ///
    /// let five_ = unsafe { TestBox::from_raw_alloc(raw, alloc) };
    /// ```
    pub fn into_raw(mut tb: Self) -> *mut T {
        let ptr = tb.ptr;
        tb.ptr = core::ptr::null_mut();
        ptr
    }
}

impl<T, A> AsRef<T> for TestBox<T, A>
where
    A: GlobalAlloc,
{
    fn as_ref(&self) -> &T {
        &*self
    }
}

impl<T, A> AsMut<T> for TestBox<T, A>
where
    A: GlobalAlloc,
{
    fn as_mut(&mut self) -> &mut T {
        &mut *self
    }
}

impl<T, A> Borrow<T> for TestBox<T, A>
where
    A: GlobalAlloc,
{
    fn borrow(&self) -> &T {
        &*self
    }
}

impl<T, A> BorrowMut<T> for TestBox<T, A>
where
    A: GlobalAlloc,
{
    fn borrow_mut(&mut self) -> &mut T {
        &mut *self
    }
}

impl<T, A> Deref for TestBox<T, A>
where
    A: GlobalAlloc,
{
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.ptr }
    }
}

impl<T, A> DerefMut for TestBox<T, A>
where
    A: GlobalAlloc,
{
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.ptr }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::alloc::System;

    #[test]
    fn constructor() {
        let _tb = TestBox::new(35, TestAlloc::<System>::default());
    }

    #[test]
    fn leak() {
        let alloc = TestAlloc::<System>::default();
        let tb = TestBox::new("foo".to_string(), alloc.clone());

        let s = TestBox::leak(tb);
        let ptr = s as *mut String;

        let layout = Layout::new::<String>();
        unsafe {
            ptr.drop_in_place();
            alloc.dealloc(ptr as *mut u8, layout);
        }
    }

    #[test]
    #[should_panic]
    fn leak_without_free() {
        let tb = TestBox::new("foo".to_string(), TestAlloc::<System>::default());

        let s = TestBox::leak(tb);
        let ptr = s as *mut String;
        unsafe { ptr.drop_in_place() };
    }

    #[test]
    fn into_raw() {
        let alloc = TestAlloc::<System>::default();
        let tb = TestBox::new("foo".to_string(), alloc.clone());

        let ptr = TestBox::into_raw(tb);

        let layout = Layout::new::<String>();
        unsafe {
            ptr.drop_in_place();
            alloc.dealloc(ptr as *mut u8, layout);
        }
    }

    #[test]
    #[should_panic]
    fn into_raw_without_free() {
        let tb = TestBox::new("foo".to_string(), TestAlloc::<System>::default());

        let ptr = TestBox::into_raw(tb);
        unsafe { ptr.drop_in_place() };
    }

    #[test]
    fn clone() {
        let tb = TestBox::new(35, TestAlloc::<System>::default());
        let _cloned = tb.clone();
    }
}
