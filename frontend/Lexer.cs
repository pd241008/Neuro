using System;
using System.Collections.Generic;

namespace Neuro.Frontend
{
    public enum TokenType
    {
        Fn, Mut, Let,
        Int, Float, Bool, String,
        If, Else, While, Return,
        True, False,
        Num, FloatNum, StrLiteral,
        Id, Assign, SemiColon,
        LBrace, RBrace, LParen, RParen, Comma,
        Equal, NotEqual, LessThan, GreaterThan, Leq, Geq,
        Plus, Minus, Multiply, Divide,
        Not, LogicalAnd, LogicalOr,
        Colon,
        EOF, Unknown
    }

    public class Token
    {
        public TokenType Type { get; set; }
        public string Value { get; set; }
        public int Line { get; set; }
        public int Column { get; set; }
        public int Offset { get; set; }

        public Token(TokenType type, string value, int line, int column, int offset = 0) {
            Type = type;
            Value = value;
            Line = line;
            Column = column;
            Offset = offset;
        }
    }

    public class Lexer {
        private readonly string _source;
        private int _pos = 0;
        private int _line = 1;
        private int _col = 1;

        private readonly Dictionary<string, TokenType> _keywords = new() {
            { "int", TokenType.Int }, { "float", TokenType.Float },
            { "bool", TokenType.Bool }, { "string", TokenType.String },
            { "if", TokenType.If }, { "else", TokenType.Else },
            { "while", TokenType.While }, { "return", TokenType.Return },
            { "true", TokenType.True }, { "false", TokenType.False },
            { "fn", TokenType.Fn }, { "mut", TokenType.Mut },
            { "let", TokenType.Let }
        };

        public Lexer(string source) => _source = source;

        public List<Token> Tokenize() {
            var tokens = new List<Token>();
            while (_pos < _source.Length) {
                char current = _source[_pos];
                int startLine = _line, startCol = _col, startPos = _pos;

                if (char.IsWhiteSpace(current)) {
                    if (current == '\n') { _line++; _col = 1; }
                    else { _col++; }
                    _pos++;
                    continue;
                }

                if (char.IsLetter(current) || current == '_') {
                    string word = ReadWhile(c => char.IsLetterOrDigit(c) || c == '_');
                    TokenType kw = _keywords.GetValueOrDefault(word, TokenType.Id);
                    tokens.Add(new Token(kw, word, startLine, startCol, startPos));
                    continue;
                }

                if (char.IsDigit(current)) {
                    string num = ReadWhile(c => char.IsDigit(c));
                    if (_pos < _source.Length && _source[_pos] == '.'
                        && _pos + 1 < _source.Length && char.IsDigit(_source[_pos + 1]))
                    {
                        _pos++; _col++;
                        num += "." + ReadWhile(c => char.IsDigit(c));
                        tokens.Add(new Token(TokenType.FloatNum, num, startLine, startCol, startPos));
                    }
                    else
                    {
                        tokens.Add(new Token(TokenType.Num, num, startLine, startCol, startPos));
                    }
                    continue;
                }

                if (current == '"') {
                    _pos++; _col++;
                    string str = "";
                    while (_pos < _source.Length && _source[_pos] != '"') {
                        if (_source[_pos] == '\\' && _pos + 1 < _source.Length) {
                            _pos++; _col++;
                            str += _source[_pos] switch {
                                'n' => '\n',
                                't' => '\t',
                                '\\' => '\\',
                                '"' => '"',
                                _ => _source[_pos]
                            };
                        } else {
                            str += _source[_pos];
                        }
                        _pos++; _col++;
                    }
                    if (_pos < _source.Length) { _pos++; _col++; }
                    tokens.Add(new Token(TokenType.StrLiteral, str, startLine, startCol, startPos));
                    continue;
                }

                if (current == '/' && _pos + 1 < _source.Length && _source[_pos + 1] == '/') {
                    while (_pos < _source.Length && _source[_pos] != '\n') {
                        if (_source[_pos] == '\n') { _line++; _col = 1; }
                        else { _col++; }
                        _pos++;
                    }
                    continue;
                }

                tokens.Add(MatchSymbol(startLine, startCol, startPos));
            }
            tokens.Add(new Token(TokenType.EOF, "", _line, _col, _pos));
            return tokens;
        }

        private string ReadWhile(Predicate<char> condition) {
            int start = _pos;
            while (_pos < _source.Length && condition(_source[_pos])) {
                _pos++;
                _col++;
            }
            return _source[start.._pos];
        }

        private Token MatchSymbol(int line, int col, int offset) {
            char c = _source[_pos++]; _col++;
            return c switch {
                '=' => Peek('=')
                    ? Consume('=', TokenType.Equal, line, col, offset)
                    : new Token(TokenType.Assign, "=", line, col, offset),
                '<' => Peek('=')
                    ? Consume('=', TokenType.Leq, line, col, offset)
                    : new Token(TokenType.LessThan, "<", line, col, offset),
                '>' => Peek('=')
                    ? Consume('=', TokenType.Geq, line, col, offset)
                    : new Token(TokenType.GreaterThan, ">", line, col, offset),
                '!' => Peek('=')
                    ? Consume('=', TokenType.NotEqual, line, col, offset)
                    : new Token(TokenType.Not, "!", line, col, offset),
                '&' => Peek('&')
                    ? Consume('&', TokenType.LogicalAnd, line, col, offset)
                    : new Token(TokenType.Unknown, "&", line, col, offset),
                '|' => Peek('|')
                    ? Consume('|', TokenType.LogicalOr, line, col, offset)
                    : new Token(TokenType.Unknown, "|", line, col, offset),
                '+' => new Token(TokenType.Plus, "+", line, col, offset),
                '-' => new Token(TokenType.Minus, "-", line, col, offset),
                '*' => new Token(TokenType.Multiply, "*", line, col, offset),
                '/' => new Token(TokenType.Divide, "/", line, col, offset),
                '(' => new Token(TokenType.LParen, "(", line, col, offset),
                ')' => new Token(TokenType.RParen, ")", line, col, offset),
                '{' => new Token(TokenType.LBrace, "{", line, col, offset),
                '}' => new Token(TokenType.RBrace, "}", line, col, offset),
                ';' => new Token(TokenType.SemiColon, ";", line, col, offset),
                ',' => new Token(TokenType.Comma, ",", line, col, offset),
                ':' => new Token(TokenType.Colon, ":", line, col, offset),
                _ => throw new Exception($"Unknown character '{c}' at line {_line}, col {_col}")
            };
        }

        private bool Peek(char expected) => _pos < _source.Length && _source[_pos] == expected;

        private Token Consume(char expected, TokenType type, int line, int col, int offset) {
            _pos++; _col++;
            return new Token(type, "", line, col, offset);
        }
    }
}
