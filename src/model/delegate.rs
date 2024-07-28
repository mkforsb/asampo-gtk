// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

macro_rules! delegate {
    // delegate!(field, fname(p0: t0, p1: t1, ..., pn: tn) -> Result)
    ($subm:ident, $fname:ident ($($parm:ident: $tp:ty),*) -> Result) => {
        pub fn $fname(self, $($parm: $tp),*) -> Result<AppModel, anyhow::Error> {
            Ok(AppModel {
                $subm: self.$subm.$fname($($parm),*)?,
                ..self
            })
        }
    };

    // delegate!(field, fname(p0: t0, p1: t1, ..., pn: tn) -> Model)
    ($subm:ident, $fname:ident ($($parm:ident: $tp:ty),*) -> Model) => {
        pub fn $fname(self, $($parm: $tp),*) -> AppModel {
            AppModel {
                $subm: self.$subm.$fname($($parm),*),
                ..self
            }
        }
    };

    // delegate!(field, fname(p0: t0, p1: t1, ..., pn: tn) -> arbitrary type)
    ($subm:ident, $fname:ident ($($parm:ident: $tp:ty),*) -> $rt:ty) => {
        pub fn $fname(&self, $($parm: $tp),*) -> $rt {
            self.$subm.$fname($($parm),*)
        }
    };

    // delegate!(field, fname(p0: t0, p1: t1, ..., pn: tn) as renamed -> Result)
    ($subm:ident, $fname:ident ($($parm:ident: $tp:ty),*) as $name:ident -> Result) => {
        pub fn $name(self, $($parm: $tp),*) -> Result<AppModel, anyhow::Error> {
            Ok(AppModel {
                $subm: self.$subm.$fname($($parm),*)?,
                ..self
            })
        }
    };

    // delegate!(field, fname(p0: t0, p1: t1, ..., pn: tn) as renamed -> Model)
    ($subm:ident, $fname:ident ($($parm:ident: $tp:ty),*) as $name:ident -> Model) => {
        pub fn $name(self, $($parm: $tp),*) -> AppModel {
            AppModel {
                $subm: self.$subm.$fname($($parm),*),
                ..self
            }
        }
    };

    // delegate!(field, fname(p0: t0, p1: t1, ..., pn: tn) as renamed -> arbitrary type)
    ($subm:ident, $fname:ident ($($parm:ident: $tp:ty),*) as $name:ident -> $rt:ty) => {
        pub fn $name(&self, $($parm: $tp),*) -> $rt {
            self.$subm.$fname($($parm),*)
        }
    };

    // delegate!(field, f0(...) [as renamed0] -> ret0, f1(...) [as renamed1] -> ret1, ...)
    ($subm:ident, $($fname:ident ( $($param:ident : $tp:ty),* ) $(as $name:ident)?
        -> $rt:ident $(< $($gen:ty),* >)?),*) =>
    {
        $(
            delegate!($subm, $fname( $($param: $tp)* ) $(as $name)? -> $rt $(< $($gen),* >)?);
        )*
    };
}

macro_rules! delegate_priv {
    // delegate!(field, fname(p0: t0, p1: t1, ..., pn: tn) -> Result)
    ($subm:ident, $fname:ident ($($parm:ident: $tp:ty),*) -> Result) => {
        fn $fname(self, $($parm: $tp),*) -> Result<AppModel, anyhow::Error> {
            Ok(AppModel {
                $subm: self.$subm.$fname($($parm),*)?,
                ..self
            })
        }
    };

    // delegate!(field, fname(p0: t0, p1: t1, ..., pn: tn) -> Model)
    ($subm:ident, $fname:ident ($($parm:ident: $tp:ty),*) -> Model) => {
        fn $fname(self, $($parm: $tp),*) -> AppModel {
            AppModel {
                $subm: self.$subm.$fname($($parm),*),
                ..self
            }
        }
    };

    // delegate!(field, fname(p0: t0, p1: t1, ..., pn: tn) -> arbitrary type)
    ($subm:ident, $fname:ident ($($parm:ident: $tp:ty),*) -> $rt:ty) => {
        fn $fname(&self, $($parm: $tp),*) -> $rt {
            self.$subm.$fname($($parm),*)
        }
    };

    // delegate!(field, fname(p0: t0, p1: t1, ..., pn: tn) as renamed -> Result)
    ($subm:ident, $fname:ident ($($parm:ident: $tp:ty),*) as $name:ident -> Result) => {
        fn $name(self, $($parm: $tp),*) -> Result<AppModel, anyhow::Error> {
            Ok(AppModel {
                $subm: self.$subm.$fname($($parm),*)?,
                ..self
            })
        }
    };

    // delegate!(field, fname(p0: t0, p1: t1, ..., pn: tn) as renamed -> Model)
    ($subm:ident, $fname:ident ($($parm:ident: $tp:ty),*) as $name:ident -> Model) => {
        fn $name(self, $($parm: $tp),*) -> AppModel {
            AppModel {
                $subm: self.$subm.$fname($($parm),*),
                ..self
            }
        }
    };

    // delegate!(field, fname(p0: t0, p1: t1, ..., pn: tn) as renamed -> arbitrary type)
    ($subm:ident, $fname:ident ($($parm:ident: $tp:ty),*) as $name:ident -> $rt:ty) => {
        fn $name(&self, $($parm: $tp),*) -> $rt {
            self.$subm.$fname($($parm),*)
        }
    };

    // delegate!(field, f0(...) [as renamed0] -> ret0, f1(...) [as renamed1] -> ret1, ...)
    ($subm:ident, $($fname:ident ( $($param:ident : $tp:ty),* ) $(as $name:ident)?
        -> $rt:ident $(< $($gen:ty),* >)?),*) =>
    {
        $(
            delegate!($subm, $fname( $($param: $tp)* ) $(as $name)? -> $rt $(< $($gen),* >)?);
        )*
    };
}

pub(in crate::model) use delegate;
pub(in crate::model) use delegate_priv;
