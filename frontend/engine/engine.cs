using System;
using System.Collections.Generic;
using System.IO;

namespace Neuro.Frontend {
    public class CompilerGUI {
        public static void Main(string[] args) {
            // Batch mode: called from the Rust Orchestrator
            if (args.Length > 0) {
                string filePath = args[0];
                if (!File.Exists(filePath)) {
                    Console.WriteLine($"Error: File not found -> {filePath}");
                    Environment.Exit(1);
                }

                string source = File.ReadAllText(filePath);
                RunCompilerPipeline(source);
                return;
            }

            // Interactive mode: GUI / REPL
            Console.WriteLine("=========================================");
            Console.WriteLine("        🧠 Welcome To Neuro 🧠         ");
            Console.WriteLine("   The Zero-Trust Compiler Pipeline    ");
            Console.WriteLine("      Type 'exit' to terminate.        ");
            Console.WriteLine("=========================================");

            while (true) {
                Console.Write("neuro> ");
                string input = Console.ReadLine();

                if (string.IsNullOrWhiteSpace(input)) continue;
                if (input.Trim().ToLower() == "exit" || input.Trim().ToLower() == "quit") break;

                RunCompilerPipeline(input, interactive: true);
            }
        }

        private static void RunCompilerPipeline(string source, bool interactive = false) {
            try {
                Lexer lexer = new Lexer(source);
                List<Token> tokens = lexer.Tokenize();

                if (interactive) {
                    Console.ForegroundColor = ConsoleColor.Cyan;
                    Console.WriteLine("[Lexer Output]");
                    foreach (var token in tokens) {
                        if (token.Type == TokenType.EOF) continue;
                        Console.WriteLine($"  {token.Type} -> '{token.Value}'");
                    }
                    Console.ResetColor();
                }

                try {
                    Parser parser = new Parser(tokens);
                    var program = parser.ParseProgram();
                    
                    if (interactive) {
                        Console.ForegroundColor = ConsoleColor.Green;
                        Console.WriteLine("✅ Parsing Successful! AST generated.");
                        Console.ResetColor();
                    } else {
                        // Serialize AST for Phase 4 (Rust Analyzer)
                        using var output = File.Create("output.ast");
                        Google.Protobuf.MessageExtensions.WriteTo(program, output);
                    }
                } catch (ParseException pEx) {
                    if (interactive) {
                        Console.ForegroundColor = ConsoleColor.Yellow;
                        Console.WriteLine($"[Parser Failed] {pEx.Message}");
                        Console.WriteLine("Note: Parser expects a full program like: int id() { }");
                        Console.ResetColor();
                    } else {
                        Console.WriteLine($"[Parser Error] {pEx.Message}");
                        Environment.Exit(1);
                    }
                }

            } catch (Exception ex) {
                if (interactive) {
                    Console.ForegroundColor = ConsoleColor.Red;
                    Console.WriteLine($"[Lexer Error] {ex.Message}");
                    Console.ResetColor();
                } else {
                    Console.WriteLine($"[Lexer Error] {ex.Message}");
                    Environment.Exit(1);
                }
            }
        }
    }
}
