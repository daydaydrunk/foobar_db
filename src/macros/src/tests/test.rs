extern crate macors;

use macors::Token;

#[derive(Token)]
enum MyEnum {
    #[token("token1")]
    Variant1,
    #[token("token2")]
    Variant2,
    #[token("token3")]
    Variant3,
}

#[test]
fn test_token_macro() {
    let v1 = MyEnum::Variant1;
    let v2 = MyEnum::Variant2;
    let v3 = MyEnum::Variant3;

    assert_eq!(v1.token(), "token1");
    assert_eq!(v2.token(), "token2");
    assert_eq!(v3.token(), "token3");
}