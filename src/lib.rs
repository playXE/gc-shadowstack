//! Shadow stack implementation for rooting in GCs
//!
//!
//! # Description
//! Unlike other algorithms to take care of rooted objects like use reference counting to take count of instances
//! on stack, this algorithm maintains a singly linked list of stack roots. This so-called "shadow stack" mirrors the
//! machine stack. Maintaining this data is much faster and memory-efficent than using reference-counted stack roots,
//! it does not require heap allocation, and does not rely on compiler optimizations.
#![no_std]

pub use mopa;
pub use paste;

/// Instatiate shadow stack type that can work with your GC API.
///
/// # Paramters
/// -`$name`: Shadow stack type name itself.
/// -`$traceable`: Your GC traceable trait that is implemented for all types that can be traced.
/// -`$rootable`: New trait name that should be implemented for rooted values.
/// -`$rooted`: Type of rooted value.
/// -`$handle`: Type of reference to rooted value. This creates `$handle` and `$handle Mut` types.
/// -`%letroot`: Name that is given to macro that will instantiate rooted values.
///
#[macro_export]
macro_rules! gc_shadowstack {
    ($name: ident,$traceable: path,$rootable: ident,$rooted: ident,$handle: ident,$letroot: ident) => {

        $crate::paste::paste!(

            /// Shadow stack implementation. Internally this is singly-linked list of on stack rooted values.
            pub struct $name {
                #[doc(hidden)]
                pub head: core::cell::Cell<*mut [<Raw $name Entry>]>,
            }


            impl $name {
                /// Create new shadow stack instance.
                pub fn new() -> Self {
                    Self {
                        head: core::cell::Cell::new(core::ptr::null_mut())
                    }
                }
                /// Walk all rooted values in this shadow stack.
                ///
                /// # Safety
                /// TODO: I don't really know if this method should be safe or unsafe.
                ///
                pub unsafe fn walk(&self,mut visitor: impl FnMut(&mut dyn $rootable)) {
                    let mut head = *self.head.as_ptr();
                    while !head.is_null() {
                        let next = (*head).prev;
                        visitor((*head).get_dyn());
                        head = next;
                    }
                }
            }


        );

        $crate::paste::paste!{
            /// Raw entry in GC shadow stack. Internal fields is not exposed in public API in any ways.
            ///
            ///
            /// This type internally stores shadow stack pointeter,previous pointer from the list and vtable
            /// that is used to construct `dyn` trait.
            ///
            #[repr(C)]
            pub struct [<Raw $name Entry>] {
                /// Shadowstack itself
                stack: *mut $name,
                /// Previous rooted entry
                prev: *mut [<Raw $name Entry>],
                /// Pointer to vtable that is a `Trace` of rooted variable
                vtable: usize,
                /// Value is located right after vtable pointer, to access it we can construct trait object.
                data_start: [u8; 0],
            }
        }
        /// Trait that should be implemented for all types that could be rooted.
        /// In simple cases `impl<T: TRace> Rootable for T {}` is enough.
        pub trait $rootable: $traceable {}
        $crate::paste::paste!(
            impl [<Raw $name Entry>] {
                /// Obtain mutable reference to rooted value.
                ///
                /// # Safety
                /// This method is `&self` but returns `&mut dyn` which is *very* unsafey. If moving GC uses shadow stack
                /// it should be ***very*** accurate when moving objects around.
                pub unsafe fn get_dyn(&self) -> &mut dyn $rootable {
                    core::mem::transmute($crate::mopa::TraitObject {
                        vtable: self.vtable as _,
                        data: self.data_start.as_ptr() as *mut (),
                    })
                }
            }

            /// Almost the same as raw entry of shadow stack except this one gives access to value.
            /// This type is not exposed in public API and used only internally.
            #[repr(C)]
            pub struct [<$name Internal>]<'a,T: $rootable> {
                pub stack :&'a $name,
                pub prev: *mut [<Raw $name Entry>],
                pub vtable: usize,
                pub value: T
            }

           impl<'a, T:$rootable> [<$name Internal>]<'a, T> {
                /// Constructs internal shadow stack value. Must not be used outside of `$letroot!` macro.
                #[inline]
                pub unsafe fn construct(
                    stack: &'a ShadowStack,
                    prev: *mut [<Raw $name Entry>],
                    vtable: usize,
                    value: T,
                ) -> Self {
                    Self {
                        stack,
                        prev,
                        vtable,
                        value,
                    }
                }
            }

            impl<T: $rootable> Drop for [<$name Internal>]<'_,T> {
                /// Drop current shadow stack entry and update shadow stack state.
                fn drop(&mut self) {
                    (*self.stack).head.set(self.prev);
                }
            }

            /// Rooted value on stack. This is non-copyable type that is used to hold GC thing on stack.
            pub struct $rooted<'a, 'b, T: $rootable> {
                #[doc(hidden)]
                pinned: core::pin::Pin<&'a mut [<$name Internal>]<'b, T>>,
            }

            impl<'a, 'b, T: $rootable> $rooted<'a, 'b, T> {
                /// Create `Rooted<T>` instance from pinned reference. Note that this should be used only
                /// inside `root!` macro and users of Starlight API should not use this function.
                pub unsafe fn construct(pin: core::pin::Pin<&'a mut [< $name Internal>]<'b, T>>) -> Self {
                    Self { pinned: pin }
                }
                pub unsafe fn get_internal(&self) -> &[<$name Internal>]<T> {
                    core::mem::transmute_copy::<_, _>(&self.pinned)
                }
                pub unsafe fn get_internal_mut(&mut self) -> &mut  &[<$name Internal>]<T> {
                    core::mem::transmute_copy::<_, _>(&self.pinned)
                }

                pub fn mut_handle(&mut self) -> [<$handle Mut>]<'_, T> {
                    HandleMut { value: &mut **self }
                }

                pub fn handle(&self) -> $handle<'_, T> {
                    Handle { value: &**self }
                }
            }

            impl<'a, T: $rootable> core::ops::Deref for $rooted<'a, '_, T> {
                type Target = T;
                fn deref(&self) -> &Self::Target {
                    &self.pinned.value
                }
            }

            impl<'a, T: $rootable> core::ops::DerefMut for $rooted<'a, '_, T> {
                fn deref_mut(&mut self) -> &mut Self::Target {
                    unsafe {
                        &mut core::mem::transmute_copy::<_, &mut [<$name Internal>]<T>>(&mut self.pinned).value
                    }
                }
            }

            /// Reference to rooted value.
            pub struct $handle<'a, T: $rootable> {
                value: &'a T,
            }
            /// Mutable reference to rooted value.
            pub struct [<$handle Mut>]<'a, T: $rootable> {
                value: &'a mut T,
            }

            impl<'a, T: $rootable> [<$handle Mut>]<'a, T> {
                pub fn set(&mut self, value: T) -> T {
                    core::mem::replace(self.value, value)
                }
            }
            impl<T: $rootable> core::ops::Deref for $handle<'_, T> {
                type Target = T;
                fn deref(&self) -> &Self::Target {
                    self.value
                }
            }

            impl<T: $rootable> core::ops::Deref for [<$handle Mut>]<'_, T> {
                type Target = T;
                fn deref(&self) -> &Self::Target {
                    self.value
                }
            }
            impl<T: $rootable> core::ops::DerefMut for [<$handle Mut>]<'_, T> {
                fn deref_mut(&mut self) -> &mut Self::Target {
                    self.value
                }
            }


            /// Create rooted value and push it to provided shadowstack instance.
            ///
            ///
            /// ***NOTE***: This macro does not heap allocate internally. It uses some unsafe tricks to
            /// allocate value on stack and push stack reference to shadowstack. Returned rooted value internally
            /// is `Pin<&mut T>`.
            ///
            #[macro_export]
            macro_rules! $letroot {
                ($var_name: ident: $t: ty  = $stack: expr,$value: expr) => {
                    let stack: &$name = &$stack;
                    let value = $value;
                    let mut $var_name = unsafe {
                        [<$name Internal>]::<$t>::construct(
                            stack,
                            stack.head.get(),
                            core::mem::transmute::<_, $crate::mopa::TraitObject>(&value as &dyn $rootable)
                                .vtable as usize,
                            value,
                        )
                    };

                    stack.head.set(unsafe { core::mem::transmute(&mut $var_name) });
                    #[allow(unused_mut)]
                    let mut $var_name =
                        unsafe { $rooted::construct(std::pin::Pin::new(&mut $var_name)) };
                };

                ($var_name : ident = $stack: expr,$value: expr) => {
                    let stack: &$name = &$stack;
                    let value = $value;
                    let mut $var_name = unsafe {
                       [<$name Internal>]::<_>::construct(
                            stack,
                            stack.head.get(),
                            core::mem::transmute::<_, $crate::mopa::TraitObject>(&value as &dyn $rootable)
                                .vtable as usize,
                            value,
                        )
                    };

                    stack.head.set(unsafe { core::mem::transmute(&mut $var_name) });
                    #[allow(unused_mut)]
                    let mut $var_name =
                        unsafe { $rooted::construct(core::pin::Pin::new(&mut $var_name)) };
                };
            }
        );
    };
}
