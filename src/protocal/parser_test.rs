use crate::protocal::parser::{ParseError, Parser};
use crate::protocal::resp::RespValue;
use std::borrow::Cow;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_string() {
        let mut parser = Parser::new(100, 1000);

        // 基本情况
        parser.read_buf(b"+OK\r\n");
        let result = match parser.try_parse() {
            Ok(Some(val)) => val,
            Ok(None) => panic!("Expected complete value"),
            Err(e) => panic!("Parse error: {:?}", e),
        };
        assert_eq!(result, RespValue::SimpleString(Cow::Borrowed("OK")));

        // 注意：Simple String 不应该包含 CR 或 LF
        // 这些应该使用 Bulk String 来传输
        parser.read_buf(b"+Hello World\r\n"); // 正确
        let result = match parser.try_parse() {
            Ok(Some(val)) => val,
            Ok(None) => panic!("Expected complete value"),
            Err(e) => panic!("Parse error: {:?}", e),
        };
        assert_eq!(
            result,
            RespValue::SimpleString(Cow::Borrowed("Hello World"))
        );

        // 测试其他合法特殊字符
        parser.read_buf(b"+Hello@#$%^&*()\r\n");
        let result = match parser.try_parse() {
            Ok(Some(val)) => val,
            Ok(None) => panic!("Expected complete value"),
            Err(e) => panic!("Parse error: {:?}", e),
        };
        assert_eq!(
            result,
            RespValue::SimpleString(Cow::Borrowed("Hello@#$%^&*()"))
        );
    }

    #[test]
    fn test_error() {
        let mut parser = Parser::new(100, 1000);

        // 基本错误
        parser.read_buf(b"-Error message\r\n");
        let result = match parser.try_parse() {
            Ok(Some(val)) => val,
            Ok(None) => panic!("Expected complete value"),
            Err(e) => panic!("Parse error: {:?}", e),
        };
        assert_eq!(result, RespValue::Error(Cow::Borrowed("Error message")));

        // 空错误
        parser.read_buf(b"-\r\n");
        let result = match parser.try_parse() {
            Ok(Some(val)) => val,
            Ok(None) => panic!("Expected complete value"),
            Err(e) => panic!("Parse error: {:?}", e),
        };
        assert_eq!(result, RespValue::Error(Cow::Borrowed("")));

        // Redis 风格错误
        parser.read_buf(b"-ERR unknown command 'foobar'\r\n");
        let result = match parser.try_parse() {
            Ok(Some(val)) => val,
            Ok(None) => panic!("Expected complete value"),
            Err(e) => panic!("Parse error: {:?}", e),
        };
        assert_eq!(
            result,
            RespValue::Error(Cow::Borrowed("ERR unknown command 'foobar'"))
        );
    }

    #[test]
    fn test_integer() {
        let mut parser = Parser::new(100, 1000);

        // 正数
        parser.read_buf(b":1234\r\n");
        let result = match parser.try_parse() {
            Ok(Some(val)) => val,
            Ok(None) => panic!("Expected complete value"),
            Err(e) => panic!("Parse error: {:?}", e),
        };
        assert_eq!(result, RespValue::Integer(1234));

        // 负数
        parser.read_buf(b":-1234\r\n");
        let result = match parser.try_parse() {
            Ok(Some(val)) => val,
            Ok(None) => panic!("Expected complete value"),
            Err(e) => panic!("Parse error: {:?}", e),
        };
        assert_eq!(result, RespValue::Integer(-1234));

        // 零
        parser.read_buf(b":0\r\n");
        let result = match parser.try_parse() {
            Ok(Some(val)) => val,
            Ok(None) => panic!("Expected complete value"),
            Err(e) => panic!("Parse error: {:?}", e),
        };
        assert_eq!(result, RespValue::Integer(0));

        // 最大值
        parser.read_buf(format!(":{}\r\n", i64::MAX).as_bytes());
        let result = match parser.try_parse() {
            Ok(Some(val)) => val,
            Ok(None) => panic!("Expected complete value"),
            Err(e) => panic!("Parse error: {:?}", e),
        };
        assert_eq!(result, RespValue::Integer(i64::MAX));

        // 最小值
        parser.read_buf(format!(":{}\r\n", i64::MIN).as_bytes());
        let result = match parser.try_parse() {
            Ok(Some(val)) => val,
            Ok(None) => panic!("Expected complete value"),
            Err(e) => panic!("Parse error: {:?}", e),
        };
        assert_eq!(result, RespValue::Integer(i64::MIN));
    }

    #[test]
    fn test_invalid_type_marker() {
        let mut parser = Parser::new(100, 1000);
        parser.read_buf(b"x1234");
        match parser.try_parse() {
            Err(ParseError::InvalidFormat(_)) => (), // Expected error
            other => panic!("Expected InvalidFormat error, got {:?}", other),
        }
    }

    #[test]
    fn test_invalid_length() {
        let mut parser = Parser::new(100, 1000);
        parser.read_buf(b"$-2");
        match parser.try_parse() {
            Err(ParseError::UnexpectedEof) => (), // Waiting for more data
            other => panic!("Expected None for incomplete data, got {:?}", other),
        }

        parser.read_buf(b"\r\n");
        match parser.try_parse() {
            Ok(Some(RespValue::Null)) => (), // Expected error
            other => panic!("Expected InvalidLength error, got {:?}", other),
        }
    }

    #[test]
    fn test_array_length_mismatch() {
        let mut parser = Parser::new(100, 1000);
        parser.read_buf(b"*2\r\n+OK\r\n");
        match parser.try_parse() {
            Err(ParseError::UnexpectedEof) => (), // Expected incomplete state
            other => panic!("Expected None for incomplete array, got {:?}", other),
        }
    }

    #[test]
    fn test_invalid_integer_format() {
        let mut parser = Parser::new(100, 1000);
        parser.read_buf(b":12.34");
        match parser.try_parse() {
            Err(ParseError::UnexpectedEof) => (), // Waiting for more data
            other => panic!("Expected None for incomplete data, got {:?}", other),
        }

        parser.read_buf(b"\r\n");
        match parser.try_parse() {
            Err(ParseError::InvalidFormat(_)) => (), // Expected error
            other => panic!("Expected InvalidFormat error, got {:?}", other),
        }
    }

    #[test]
    fn test_missing_crlf() {
        let mut parser = Parser::new(100, 1000);
        parser.read_buf(b"+OK\n");
        match parser.try_parse() {
            Err(ParseError::UnexpectedEof) => (), // Expected error
            other => panic!("Expected InvalidFormat error, got {:?}", other),
        }
    }

    #[test]
    fn test_exceeding_maximum_depth() {
        let mut shallow_parser = Parser::new(1, 1000);
        shallow_parser.read_buf(b"*1\r\n");
        match shallow_parser.try_parse() {
            Err(ParseError::UnexpectedEof) => (), // Waiting for more data
            other => panic!("Expected None for incomplete data, got {:?}", other),
        }

        shallow_parser.read_buf(b"*1\r\n");
        match shallow_parser.try_parse() {
            Err(ParseError::InvalidDepth) => (), // Waiting for more data
            other => panic!("Expected None for incomplete data, got {:?}", other),
        }

        shallow_parser.read_buf(b"+OK\r\n");
        match shallow_parser.try_parse() {
            Err(ParseError::InvalidDepth) => (), // Expected error
            other => panic!(
                "Expected InvalidFormat error for exceeding maximum depth, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_incomplete_messages() {
        let mut parser = Parser::new(100, 1000);

        // 不完整的简单字符串
        parser.read_buf(b"+OK");
        match parser.try_parse() {
            Err(ParseError::UnexpectedEof) => (), // 等待更多数据
            other => panic!(
                "Expected None for incomplete simple string, got {:?}",
                other
            ),
        }

        // 重置解析器
        parser = Parser::new(100, 1000);

        // 不完整的错误消息
        parser.read_buf(b"-ERR");
        match parser.try_parse() {
            Err(ParseError::UnexpectedEof) => (), // 等待更多数据
            other => panic!(
                "Expected None for incomplete error message, got {:?}",
                other
            ),
        }

        // 重置解析器
        parser = Parser::new(100, 1000);

        // 不完整的整数
        parser.read_buf(b":123");
        match parser.try_parse() {
            Err(ParseError::UnexpectedEof) => (), // 等待更多数据
            other => panic!("Expected None for incomplete integer, got {:?}", other),
        }

        // 重置解析器
        parser = Parser::new(100, 1000);

        // 不完整的批量字符串长度
        parser.read_buf(b"$5");
        match parser.try_parse() {
            Err(ParseError::UnexpectedEof) => (), // 等待更多数据
            other => panic!(
                "Expected None for incomplete bulk string length, got {:?}",
                other
            ),
        }

        // 重置解析器
        parser = Parser::new(100, 1000);

        // 不完整的数组长度
        parser.read_buf(b"*3");
        match parser.try_parse() {
            Err(ParseError::UnexpectedEof) => (), // 等待更多数据
            other => panic!("Expected None for incomplete array length, got {:?}", other),
        }
    }

    #[test]
    fn test_large_messages() {
        let mut parser = Parser::new(100, 10000);

        // 大字符串
        let large_string = "x".repeat(1000);
        let message = format!("${}\r\n{}\r\n", large_string.len(), large_string);

        // 分段发送长度信息
        parser.read_buf(format!("${}\r\n", large_string.len()).as_bytes());
        match parser.try_parse() {
            Err(ParseError::NotEnoughData) => (), // 期望继续等待更多数据
            other => panic!("Expected None, got {:?}", other),
        }

        // 分发送数据
        let chunks = large_string.as_bytes().chunks(100);
        for chunk in chunks {
            parser.read_buf(chunk);
            match parser.try_parse() {
                Err(ParseError::NotEnoughData) => (), // 期望继续等待更多数据
                other => panic!("Expected None, got {:?}", other),
            }
        }

        // 发送结束符
        parser.read_buf(b"\r\n");
        match parser.try_parse() {
            Ok(Some(RespValue::BulkString(Some(msg)))) => {
                assert_eq!(msg, large_string);
            }
            other => panic!("Expected BulkString, got {:?}", other),
        }

        // 大数组
        let mut large_array = String::from("*1000\r\n");
        parser.read_buf(large_array.as_bytes());
        match parser.try_parse() {
            Err(ParseError::UnexpectedEof) => (), // 期望继续等待更多数据
            other => panic!("Expected None, got {:?}", other),
        }

        // 分段发送数组元素
        for _ in 0..999 {
            parser.read_buf(b":1\r\n");
            match parser.try_parse() {
                Err(ParseError::UnexpectedEof) => (), // 期望继续等待更多数据
                other => panic!("Expected None, got {:?}", other),
            }
        }

        // 发送最后一个元素
        parser.read_buf(b":1\r\n");
        match parser.try_parse() {
            Ok(Some(RespValue::Array(Some(arr)))) => {
                assert_eq!(arr.len(), 1000);
                assert!(arr.iter().all(|x| *x == RespValue::Integer(1)));
            }
            other => panic!("Expected Array, got {:?}", other),
        }
    }

    #[test]
    fn test_error_message_chunks() {
        let mut parser = Parser::new(100, 1000);

        // 第一段：只有错类型标记和部分消息
        parser.read_buf(b"-ERR unknow");
        match parser.try_parse() {
            Err(ParseError::UnexpectedEof) => (), // 期望继续等待更多数据
            other => panic!("Expected None, got {:?}", other),
        }

        // 第二段：继续添加消息
        parser.read_buf(b"n command");
        match parser.try_parse() {
            Err(ParseError::UnexpectedEof) => (), // 期望继续等待更多数据
            other => panic!("Expected None, got {:?}", other),
        }

        // 第三段：添加结束符
        parser.read_buf(b"\r\n");
        match parser.try_parse() {
            Ok(Some(RespValue::Error(msg))) => {
                assert_eq!(msg, "ERR unknown command");
            }
            other => panic!("Expected Error message, got {:?}", other),
        }
    }

    #[test]
    fn test_bulk_string_chunks() {
        // 测试完整输入
        {
            let mut parser = Parser::new(100, 1000);
            parser.read_buf(b"$0\r\n\r\n");
            let result = parser.try_parse();
            assert_eq!(result, Ok(Some(RespValue::Null)));
        }

        // 测试两块数据
        {
            let mut parser = Parser::new(100, 1000);

            // 第一块：类型标记和长度
            parser.read_buf(b"$0\r\n");
            let result = parser.try_parse();
            assert_eq!(result, Ok(Some(RespValue::Null))); // 还需要更多数据

            // 第二块：结束符
            parser.read_buf(b"\r\n");
            let result = parser.try_parse();
            assert_eq!(result, Err(ParseError::UnexpectedEof));
        }

        // 测试三块数据
        {
            let mut parser = Parser::new(100, 1000);

            // 第一块：类型标记
            parser.read_buf(b"$5");
            let result = parser.try_parse();
            assert_eq!(result, Err(ParseError::UnexpectedEof));

            // 第二块：长度和数据
            parser.read_buf(b"\r\nhello");
            let result = parser.try_parse();
            assert_eq!(result, Err(ParseError::NotEnoughData));

            // 第三块：结束符
            parser.read_buf(b"\r\n");
            let result = parser.try_parse();
            assert_eq!(
                result,
                Ok(Some(RespValue::BulkString(Some(Cow::Borrowed("hello")))))
            );
        }

        // 测试非空字符串的分块传输
        {
            let mut parser = Parser::new(100, 1000);

            // 第一块：头部
            parser.read_buf(b"$12\r\n");
            let result = parser.try_parse();
            assert_eq!(result, Err(ParseError::NotEnoughData));

            // 第二块：部分数据
            parser.read_buf(b"Hello ");
            let result = parser.try_parse();
            assert_eq!(result, Err(ParseError::NotEnoughData));

            // 第三块：剩余数据
            parser.read_buf(b"World!");
            let result = parser.try_parse();
            assert_eq!(result, Err(ParseError::NotEnoughData));

            // 第四块：结束符
            parser.read_buf(b"\r\n");
            let result = parser.try_parse();
            assert_eq!(
                result,
                Ok(Some(RespValue::BulkString(Some(Cow::Borrowed(
                    "Hello World!"
                )))))
            );
        }
    }

    #[test]
    fn test_array_chunks() {
        // 测试简单数组的分块传输
        {
            let mut parser = Parser::new(100, 1000);

            // 第一块：数组长度
            parser.read_buf(b"*2");
            _ = parser.try_parse();

            // 第二块：数组长度结束符和第一个元素开始
            parser.read_buf(b"\r\n:1");
            _ = parser.try_parse();

            // 第三块：第一个元素结束符
            parser.read_buf(b"\r\n");
            _ = parser.try_parse();

            // 第四块：第二个元素
            parser.read_buf(b":2\r\n");
            let result = parser.try_parse();
            assert_eq!(
                result,
                Ok(Some(RespValue::Array(Some(vec![
                    RespValue::Integer(1),
                    RespValue::Integer(2)
                ]))))
            );
        }

        // 测试空数组
        {
            let mut parser = Parser::new(100, 1000);
            parser.read_buf(b"*0\r\n");
            let result = parser.try_parse();
            assert_eq!(result, Ok(Some(RespValue::Array(None))));
        }

        // 测试 null 数组
        {
            let mut parser = Parser::new(100, 1000);
            parser.read_buf(b"*-1\r\n");
            let result = parser.try_parse();
            assert_eq!(result, Ok(Some(RespValue::Array(None))));
        }

        // 测试混合类型数组
        {
            let mut parser = Parser::new(100, 1000);

            // 发送数组头部和第一个元素（整数）
            parser.read_buf(b"*3\r\n:123\r\n");
            _ = parser.try_parse(); // 需要更多元素

            // 发送第二个元素（简单字符串）
            parser.read_buf(b"+hello\r\n");
            _ = parser.try_parse(); // 需要更多元素

            // 发送第三个元素（批量字符串）
            parser.read_buf(b"$5\r\nworld\r\n");
            let result = parser.try_parse();
            assert_eq!(
                result,
                Ok(Some(RespValue::Array(Some(vec![
                    RespValue::Integer(123),
                    RespValue::SimpleString("hello".into()),
                    RespValue::BulkString(Some("world".into()))
                ]))))
            );
        }

        // 测试嵌套数组
        {
            let mut parser = Parser::new(100, 1000);

            // 外层数组开始
            parser.read_buf(b"*2\r\n");
            let result = parser.try_parse();
            assert_eq!(result, Err(ParseError::UnexpectedEof));

            // 内层数组1
            parser.read_buf(b"*2\r\n+a\r\n+b\r\n");
            let result = parser.try_parse();
            assert_eq!(result, Err(ParseError::UnexpectedEof));

            // 内层数组2
            parser.read_buf(b"*2\r\n+c\r\n+d\r\n");
            let result = parser.try_parse();
            assert_eq!(
                result,
                Ok(Some(RespValue::Array(Some(vec![
                    RespValue::Array(Some(vec![
                        RespValue::SimpleString("a".into()),
                        RespValue::SimpleString("b".into())
                    ])),
                    RespValue::Array(Some(vec![
                        RespValue::SimpleString("c".into()),
                        RespValue::SimpleString("d".into())
                    ]))
                ]))))
            );
        }

        // 测试大数组的分块传输
        // {
        //     let mut parser = Parser::new(100, 1000);

        //     // 发送数组长度
        //     parser.read_buf(b"*3\r\n");
        //     let result = parser.try_parse();
        //     assert_eq!(result, Err(ParseError::UnexpectedEof));

        //     // 逐个发送大量整数
        //     for i in 1..=3 {
        //         parser.read_buf(format!(":1{}\r\n", i).as_bytes());
        //         let expected = if i < 3 {
        //             Ok(None)
        //         } else {
        //             Ok(Some(RespValue::Array(Some(vec![
        //                 RespValue::Integer(11),
        //                 RespValue::Integer(12),
        //                 RespValue::Integer(13),
        //             ]))))
        //         };
        //         assert_eq!(parser.try_parse(), expected);
        //     }
        // }

        // 测试错误情况
        {
            let mut parser = Parser::new(100, 1000);

            // 无效的数组长度
            parser.read_buf(b"*-2\r\n");
            let result = parser.try_parse();
            assert_eq!(result, Ok(Some(RespValue::Array(None))));

            // 重置解析器
            parser = Parser::new(100, 1000);

            // 数组元素不完整
            parser.read_buf(b"*2\r\n:1\r\n");
            let result = parser.try_parse();
            assert_eq!(result, Err(ParseError::UnexpectedEof)); // 需要更多元素
        }

        let mut parser = Parser::new(100, 1000);

        // 空数组
        parser.read_buf(b"*0\r\n");
        assert_eq!(parser.try_parse(), Ok(Some(RespValue::Array(None))));

        // Null 数组
        parser.read_buf(b"*-1\r\n");
        assert_eq!(parser.try_parse(), Ok(Some(RespValue::Array(None))));

        // 简单数组
        parser.read_buf(b"*2\r\n$5\r\nhello\r\n$5\r\nworld\r\n");
        assert_eq!(
            parser.try_parse(),
            Ok(Some(RespValue::Array(Some(vec![
                RespValue::BulkString(Some(Cow::Borrowed("hello"))),
                RespValue::BulkString(Some(Cow::Borrowed("world")))
            ]))))
        );

        // 混合类型数组
        parser.read_buf(b"*5\r\n:1\r\n:2\r\n:3\r\n:4\r\n$5\r\nhello\r\n");
        assert_eq!(
            parser.try_parse(),
            Ok(Some(RespValue::Array(Some(vec![
                RespValue::Integer(1),
                RespValue::Integer(2),
                RespValue::Integer(3),
                RespValue::Integer(4),
                RespValue::BulkString(Some(Cow::Borrowed("hello")))
            ]))))
        );

        // 嵌套数组
        parser.read_buf(b"*2\r\n*2\r\n+a\r\n+b\r\n*2\r\n+c\r\n+d\r\n");
        assert_eq!(
            parser.try_parse(),
            Ok(Some(RespValue::Array(Some(vec![
                RespValue::Array(Some(vec![
                    RespValue::SimpleString(Cow::Borrowed("a")),
                    RespValue::SimpleString(Cow::Borrowed("b"))
                ])),
                RespValue::Array(Some(vec![
                    RespValue::SimpleString(Cow::Borrowed("c")),
                    RespValue::SimpleString(Cow::Borrowed("d"))
                ]))
            ]))))
        );
    }

    #[test]
    fn test_integer_chunks() {
        let mut parser = Parser::new(100, 1000);

        // 第一段：类型标记和部分数字
        parser.read_buf(b":123");
        match parser.try_parse() {
            Err(ParseError::UnexpectedEof) => (), // 期望继续等待更多数据
            other => panic!("Expected None, got {:?}", other),
        }

        // 第二段：剩余数字
        parser.read_buf(b"45");
        match parser.try_parse() {
            Err(ParseError::UnexpectedEof) => (), // 期望继续等待更多数据
            other => panic!("Expected None, got {:?}", other),
        }

        // 第三段：结束符
        parser.read_buf(b"\r\n");
        match parser.try_parse() {
            Ok(Some(RespValue::Integer(num))) => {
                assert_eq!(num, 12345);
            }
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[test]
    fn test_large_bulk_string_chunks() {
        let mut parser = Parser::new(100, 10000);

        // 构造大字符串
        let large_string = "x".repeat(1000);

        // 第一段：长度前缀
        parser.read_buf(format!("${}\r\n", large_string.len()).as_bytes());
        match parser.try_parse() {
            Err(ParseError::NotEnoughData) => (), // 期望继续等待更多数据
            other => panic!("Expected None, got {:?}", other),
        }

        // 分多段发送大字符串
        let chunk_size = 100;
        for chunk in large_string.as_bytes().chunks(chunk_size) {
            parser.read_buf(chunk);
            match parser.try_parse() {
                Err(ParseError::NotEnoughData) => (), // 期望继续等待多数据
                other => panic!("Expected None while processing chunks, got {:?}", other),
            }
        }

        // 最后发送结束符
        parser.read_buf(b"\r\n");
        match parser.try_parse() {
            Ok(Some(RespValue::BulkString(Some(msg)))) => {
                assert_eq!(msg, large_string);
            }
            other => panic!("Expected BulkString, got {:?}", other),
        }
    }
}
