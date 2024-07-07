// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

#![allow(unused_macros, unused_imports)]

macro_rules! delegate {
    ($subm:ident, $fname:ident ($($param0:ident: $tp0:ty $(, $param:ident: $tp:ty)*)?) -> Result) => {
        pub fn $fname(self $(, $param0: $tp0 $(, $param: $tp)*)?) -> Result<AppModel, anyhow::Error> {
            Ok(AppModel {
                $subm: self.$subm.$fname($($param0 $(, $param)*)?)?,
                ..self
            })
        }
    };
    ($subm:ident, $fname:ident ($($param0:ident: $tp0:ty $(, $param:ident: $tp:ty)*)?) -> Model) => {
        pub fn $fname(self $(, $param0: $tp0 $(, $param: $tp)*)?) -> AppModel {
            AppModel {
                $subm: self.$subm.$fname($($param0 $(, $param)*)?),
                ..self
            }
        }
    };
    ($subm:ident, $fname:ident ($($param0:ident: $tp0:ty $(, $param:ident: $tp:ty)*)?) -> $rt:ty) => {
        pub fn $fname(&self $(, $param0: $tp0 $(, $param: $tp)*)?) -> $rt {
            self.$subm.$fname($($param0 $(, $param)*)?)
        }
    };
    ($subm:ident, $fname:ident ($($param0:ident: $tp0:ty $(, $param:ident: $tp:ty)*)?) as $name:ident -> Result) => {
        pub fn $name(self $(, $param0: $tp0 $(, $param: $tp)*)?) -> Result<AppModel, anyhow::Error> {
            Ok(AppModel {
                $subm: self.$subm.$fname($($param0 $(, $param)*)?)?,
                ..self
            })
        }
    };
    ($subm:ident, $fname:ident ($($param0:ident: $tp0:ty $(, $param:ident: $tp:ty)*)?) as $name:ident -> Model) => {
        pub fn $name(self $(, $param0: $tp0 $(, $param: $tp)*)?) -> AppModel {
            AppModel {
                $subm: self.$subm.$fname($($param0 $(, $param)*)?),
                ..self
            }
        }
    };
    ($subm:ident, $fname:ident ($($param0:ident: $tp0:ty $(, $param:ident: $tp:ty)*)?) as $name:ident -> $rt:ty) => {
        pub fn $name(&self $(, $param0: $tp0 $(, $param: $tp)*)?) -> $rt {
            self.$subm.$fname($($param0 $(, $param)*)?)
        }
    };
}

macro_rules! delegate_priv {
    ($subm:ident, $fname:ident ($($param0:ident: $tp0:ty $(, $param:ident: $tp:ty)*)?) -> Result) => {
        fn $fname(self $(, $param0: $tp0 $(, $param: $tp)*)?) -> Result<AppModel, anyhow::Error> {
            Ok(AppModel {
                $subm: self.$subm.$fname($($param0 $(, $param)*)?)?,
                ..self
            })
        }
    };
    ($subm:ident, $fname:ident ($($param0:ident: $tp0:ty $(, $param:ident: $tp:ty)*)?) -> Model) => {
        fn $fname(self $(, $param0: $tp0 $(, $param: $tp)*)?) -> AppModel {
            AppModel {
                $subm: self.$subm.$fname($($param0 $(, $param)*)?),
                ..self
            }
        }
    };
    ($subm:ident, $fname:ident ($($param0:ident: $tp0:ty $(, $param:ident: $tp:ty)*)?) -> $rt:ty) => {
        fn $fname(&self $(, $param0: $tp0 $(, $param: $tp)*)?) -> $rt {
            self.$subm.$fname($($param0 $(, $param)*)?)
        }
    };
    ($subm:ident, $fname:ident ($($param0:ident: $tp0:ty $(, $param:ident: $tp:ty)*)?) as $name:ident -> Result) => {
        fn $name(self $(, $param0: $tp0 $(, $param: $tp)*)?) -> Result<AppModel, anyhow::Error> {
            Ok(AppModel {
                $subm: self.$subm.$fname($($param0 $(, $param)*)?)?,
                ..self
            })
        }
    };
    ($subm:ident, $fname:ident ($($param0:ident: $tp0:ty $(, $param:ident: $tp:ty)*)?) as $name:ident -> Model) => {
        fn $name(self $(, $param0: $tp0 $(, $param: $tp)*)?) -> AppModel {
            AppModel {
                $subm: self.$subm.$fname($($param0 $(, $param)*)?),
                ..self
            }
        }
    };
    ($subm:ident, $fname:ident ($($param0:ident: $tp0:ty $(, $param:ident: $tp:ty)*)?) as $name:ident -> $rt:ty) => {
        fn $name(&self $(, $param0: $tp0 $(, $param: $tp)*)?) -> $rt {
            self.$subm.$fname($($param0 $(, $param)*)?)
        }
    };
}

pub(in crate::model) use delegate;
pub(in crate::model) use delegate_priv;
