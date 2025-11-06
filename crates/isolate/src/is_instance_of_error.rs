// The below method was taken from `deno_core`
// https://github.com/denoland/deno_core/blob/main/LICENSE.md - MIT License
// Copyright 2018-2024 the Deno authors
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.
use deno_core::v8;

/// Implements the JS check `value instanceof Error`. Similar to
/// `Value::is_native_error()` but more closely matches the semantics
/// of `instanceof`. `Value::is_native_error()` also checks for static class
/// inheritance rather than just scanning the prototype chain, which doesn't
/// work with our WebIDL implementation of `DOMException`.
pub fn is_instance_of_error(scope: &v8::PinScope<'_, '_>, value: v8::Local<'_, v8::Value>) -> bool {
    if !value.is_object() {
        return false;
    }
    let message = v8::String::empty(scope);
    let error_prototype = v8::Exception::error(scope, message)
        .to_object(scope)
        .unwrap()
        .get_prototype(scope)
        .unwrap();
    let mut maybe_prototype = value.to_object(scope).unwrap().get_prototype(scope);
    while let Some(prototype) = maybe_prototype {
        if !prototype.is_object() {
            return false;
        }
        if prototype.strict_equals(error_prototype) {
            return true;
        }
        maybe_prototype = prototype
            .to_object(scope)
            .and_then(|o| o.get_prototype(scope));
    }
    false
}
