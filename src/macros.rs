/***********************************************************************************************************************
 * Copyright (c) 2020 by the authors
 * 
 * Author: André Borrmann <pspwizard@gmx.de>
 * License: Apache License 2.0 / MIT
 **********************************************************************************************************************/

#[macro_export]
macro_rules! define_tlb_entry {
    ($($(#[doc = $rdoc:expr])* $vis:vis $name:ident $(
        { $(
                $(#[doc = $fdoc:expr])*
                $field:ident OFFSET($offset:literal) $(BITS($bits:literal))?
                $([$($(#[doc = $fvdoc:expr])* $enum:ident = $value:expr),*])?
        ),* }
    )?),*) => {
        $(
            #[allow(non_snake_case)]
            #[allow(non_upper_case_globals)]
            $(#[doc = $rdoc])*
            $vis mod $name {
                #[allow(unused_imports)]
                use super::*;
                $(
                    $(
                        $crate::register_field!(u64, $field, $offset $(, $bits)?);
                        $(#[doc = $fdoc])*
                        pub mod $field {
                            use super::*;
                            /// Create a ``RegisterFieldValue`` from the current ``RegisterField``
                            #[inline]
                            #[allow(unused_variables, dead_code)]
                            pub const fn with_value(value: u64) -> RegisterFieldValue<u64> {
                                RegisterFieldValue::<u64>::new($field, value)
                            }
                            $(
                                $crate::register_field_values!($field, u64, $($($fvdoc)*, $enum = $value),*);
                            )*
                        }
                    )*
                )*
            }
        )*
    };
}