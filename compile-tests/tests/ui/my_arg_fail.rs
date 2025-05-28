#[plap_compile_tests::test(my_arg)]
struct UserInput {
    #[my_arg(arg1 = "value #1", arg5 = 1, arg5, arg5)]
    some_field: String,
    #[my_arg(arg1 = "value #2", arg2, arg4 = "Whatever")]
    #[my_arg(arg3 = "Vec<String>")]
    another_field: i32,
}

fn main() {}
