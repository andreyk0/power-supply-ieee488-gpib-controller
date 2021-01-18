#[macro_export()]
macro_rules! ifcfg {
    ($cc:tt, $e:expr) => {
        if core::cfg!(feature = $cc) {
            $e.unwrap()
        } else {
            ()
        }
    };
}
