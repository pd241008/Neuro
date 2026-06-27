using System.Linq;
using Xunit;
using Neuro.Frontend;
using Neuro.AST;
using Type = Neuro.AST.Type;

namespace frontend.Tests
{
    public class ParserTests
    {
        private Program Parse(string source)
        {
            var lexer = new Lexer(source);
            var tokens = lexer.Tokenize();
            var parser = new Parser(tokens);
            return parser.ParseProgram();
        }

        [Fact]
        public void Test_Parse_EmptyProgram()
        {
            var source = "";
            var program = Parse(source);

            Assert.NotNull(program);
            Assert.Empty(program.Functions);
        }

        [Fact]
        public void Test_Parse_FunctionNoArgs()
        {
            var source = "fn main() -> void {}";
            var program = Parse(source);

            Assert.Single(program.Functions);
            var func = program.Functions.First();
            Assert.Equal("main", func.Name);
            Assert.Empty(func.Parameters);
            Assert.Equal(Type.Types.Kind.Void, func.ReturnType.Kind);
            Assert.Empty(func.Body);
        }

        [Fact]
        public void Test_Parse_FunctionWithArgs()
        {
            var source = "fn add(x: int, y: int) -> int { return x + y; }";
            var program = Parse(source);

            Assert.Single(program.Functions);
            var func = program.Functions.First();
            Assert.Equal("add", func.Name);
            Assert.Equal(2, func.Parameters.Count);
            
            Assert.Equal("x", func.Parameters[0].Name);
            Assert.Equal(Type.Types.Kind.Int, func.Parameters[0].Type.Kind);
            
            Assert.Equal("y", func.Parameters[1].Name);
            Assert.Equal(Type.Types.Kind.Int, func.Parameters[1].Type.Kind);
            
            Assert.Equal(Type.Types.Kind.Int, func.ReturnType.Kind);
            
            Assert.Single(func.Body);
            Assert.NotNull(func.Body[0].ReturnStmt);
            Assert.NotNull(func.Body[0].ReturnStmt.Value);
        }

        [Fact]
        public void Test_Parse_VariableDeclaration()
        {
            var source = "fn main() -> void { let x: int = 10; let mut y: float = 3.14; }";
            var program = Parse(source);

            var func = program.Functions.First();
            Assert.Equal(2, func.Body.Count);
            
            // let x: int = 10;
            var decl1 = func.Body[0].Declaration;
            Assert.NotNull(decl1);
            Assert.Equal("x", decl1.Name);
            Assert.Equal(Type.Types.Kind.Int, decl1.Type.Kind);
            Assert.False(decl1.IsMutable);
            Assert.NotNull(decl1.Initializer.Literal);
            Assert.Equal(10, decl1.Initializer.Literal.IntVal);

            // let mut y: float = 3.14;
            var decl2 = func.Body[1].Declaration;
            Assert.NotNull(decl2);
            Assert.Equal("y", decl2.Name);
            Assert.Equal(Type.Types.Kind.Float, decl2.Type.Kind);
            Assert.True(decl2.IsMutable);
            Assert.NotNull(decl2.Initializer.Literal);
            Assert.Equal(3.14, decl2.Initializer.Literal.FloatVal, 2);
        }

        [Fact]
        public void Test_Parse_IfStatement()
        {
            var source = "fn main() -> void { if (true) { } else { } }";
            var program = Parse(source);

            var func = program.Functions.First();
            Assert.Single(func.Body);

            var ifStmt = func.Body[0].IfStmt;
            Assert.NotNull(ifStmt);
            Assert.NotNull(ifStmt.Condition.Literal);
            Assert.True(ifStmt.Condition.Literal.BoolVal);
            
            Assert.Empty(ifStmt.TrueBranch);
            Assert.Empty(ifStmt.FalseBranch);
        }

        [Fact]
        public void Test_Parse_WhileStatement()
        {
            var source = "fn main() -> void { while (x < 10) { x = x + 1; } }";
            var program = Parse(source);

            var func = program.Functions.First();
            Assert.Single(func.Body);

            var whileStmt = func.Body[0].WhileStmt;
            Assert.NotNull(whileStmt);
            
            Assert.NotNull(whileStmt.Condition.BinaryOp);
            Assert.Equal(BinaryOperation.Types.Operator.Lt, whileStmt.Condition.BinaryOp.Op);
            
            Assert.Single(whileStmt.Body);
            var assignStmt = whileStmt.Body[0].Assignment;
            Assert.NotNull(assignStmt);
            Assert.Equal("x", assignStmt.TargetName);
        }

        [Fact]
        public void Test_Parse_ExceptionOnInvalidSyntax()
        {
            var source = "fn main() -> void { let 123 = x; }";
            
            var exception = Assert.Throws<ParseException>(() => Parse(source));
            Assert.Contains("Expected 'Id'", exception.Message);
        }
    }
}
