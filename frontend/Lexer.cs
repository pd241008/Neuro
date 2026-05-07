using System;
using System.Collections.Generic;

namespace Neuro.Frontend
{
    public enum TokenType
    {
        Int, Float, If, Else, Return, Printf, Scanf,
        Leq, Id, Num, String, Assign, SemiColon,
        LBrace, RBrace, LParen, RParen, Comma, Multiply, EOF, Unknown
    }

    public class Token
    {
        public TokenType Type { get; set; }
        public string Value { get; set; }
    }

    public class Lexer
    {
        // Migrated logic from compiler/lexer.l
        // Keywords: int, float, if, else, return, printf, scanf, <=
        // Patterns: 
        //   ID: [a-zA-Z_][a-zA-Z0-9_]*
        //   NUM: [0-9]+(\.[0-9]+)?
        //   STRING: \"[^\"]*\"
        // Ignores: whitespace (\t, \n, \r, space)

        public List<Token> Tokenize(string source)
        {
            var tokens = new List<Token>();
            Console.WriteLine("Lexing source code into tokens...");
            // TODO: Implement scanner logic here based on legacy patterns
            return tokens;
        }
    }
}
