using System.Linq;
using Xunit;
using Neuro.Frontend;

namespace frontend.Tests
{
    public class LexerTests
    {
        [Fact]
        public void Test_Keywords_TokenizedCorrectly()
        {
            var source = "fn let mut int float bool string void if else while return true false";
            var lexer = new Lexer(source);
            var tokens = lexer.Tokenize();

            var expectedTypes = new[]
            {
                TokenType.Fn, TokenType.Let, TokenType.Mut, TokenType.Int,
                TokenType.Float, TokenType.Bool, TokenType.String, TokenType.Void,
                TokenType.If, TokenType.Else, TokenType.While, TokenType.Return,
                TokenType.True, TokenType.False, TokenType.EOF
            };

            Assert.Equal(expectedTypes.Length, tokens.Count);
            for (int i = 0; i < expectedTypes.Length; i++)
            {
                Assert.Equal(expectedTypes[i], tokens[i].Type);
            }
        }

        [Fact]
        public void Test_Identifiers_TokenizedCorrectly()
        {
            var source = "myVar123 _anotherVar";
            var lexer = new Lexer(source);
            var tokens = lexer.Tokenize();

            Assert.Equal(3, tokens.Count);
            Assert.Equal(TokenType.Id, tokens[0].Type);
            Assert.Equal("myVar123", tokens[0].Value);
            
            Assert.Equal(TokenType.Id, tokens[1].Type);
            Assert.Equal("_anotherVar", tokens[1].Value);
            
            Assert.Equal(TokenType.EOF, tokens[2].Type);
        }

        [Fact]
        public void Test_Numbers_TokenizedCorrectly()
        {
            var source = "42 3.14 0";
            var lexer = new Lexer(source);
            var tokens = lexer.Tokenize();

            Assert.Equal(4, tokens.Count);
            Assert.Equal(TokenType.Num, tokens[0].Type);
            Assert.Equal("42", tokens[0].Value);

            Assert.Equal(TokenType.FloatNum, tokens[1].Type);
            Assert.Equal("3.14", tokens[1].Value);

            Assert.Equal(TokenType.Num, tokens[2].Type);
            Assert.Equal("0", tokens[2].Value);
        }

        [Fact]
        public void Test_Strings_TokenizedCorrectly()
        {
            var source = "\"hello world\" \"with \\n newline\"";
            var lexer = new Lexer(source);
            var tokens = lexer.Tokenize();

            Assert.Equal(3, tokens.Count);
            Assert.Equal(TokenType.StrLiteral, tokens[0].Type);
            Assert.Equal("hello world", tokens[0].Value);
            
            Assert.Equal(TokenType.StrLiteral, tokens[1].Type);
            Assert.Equal("with \n newline", tokens[1].Value);
        }

        [Fact]
        public void Test_Operators_TokenizedCorrectly()
        {
            var source = "= == < <= > >= ! != & | + - * / : ; , ( ) { }";
            var lexer = new Lexer(source);
            var tokens = lexer.Tokenize();

            var expectedTypes = new[]
            {
                TokenType.Assign, TokenType.Equal, TokenType.LessThan, TokenType.Leq,
                TokenType.GreaterThan, TokenType.Geq, TokenType.Not, TokenType.NotEqual,
                TokenType.Unknown, TokenType.Unknown, TokenType.Plus, TokenType.Minus,
                TokenType.Multiply, TokenType.Divide, TokenType.Colon, TokenType.SemiColon,
                TokenType.Comma, TokenType.LParen, TokenType.RParen, TokenType.LBrace, TokenType.RBrace,
                TokenType.EOF
            };

            Assert.Equal(expectedTypes.Length, tokens.Count);
            for (int i = 0; i < expectedTypes.Length; i++)
            {
                Assert.Equal(expectedTypes[i], tokens[i].Type);
            }
        }

        [Fact]
        public void Test_Comments_AreIgnored()
        {
            var source = @"
                let x = 10; // This is a comment
                let y = 20; // Another comment
            ";
            var lexer = new Lexer(source);
            var tokens = lexer.Tokenize();

            // let, x, =, 10, ;, let, y, =, 20, ;, EOF = 11 tokens
            Assert.Equal(11, tokens.Count);
            Assert.True(tokens.All(t => t.Type != TokenType.Unknown));
        }

        [Fact]
        public void Test_LineAndColumn_TrackedCorrectly()
        {
            var source = "let x = 10;\nfn main() {}";
            var lexer = new Lexer(source);
            var tokens = lexer.Tokenize();

            // let x = 10; -> line 1
            Assert.Equal(1, tokens[0].Line); // let
            Assert.Equal(1, tokens[0].Column);
            
            Assert.Equal(1, tokens[1].Line); // x
            Assert.Equal(5, tokens[1].Column);

            Assert.Equal(1, tokens[4].Line); // ;
            Assert.Equal(11, tokens[4].Column);

            // fn main() {} -> line 2
            Assert.Equal(2, tokens[5].Line); // fn
            Assert.Equal(1, tokens[5].Column);
            
            Assert.Equal(2, tokens[6].Line); // main
            Assert.Equal(4, tokens[6].Column);
        }
    }
}
