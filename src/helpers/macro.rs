#[macro_export]
macro_rules! ternary_expr {
    ($cond:expr, $if_true: expr, $if_false:expr ) => {
        if $cond {
            $if_true
        } else {
            $if_false
        }
    };
}
