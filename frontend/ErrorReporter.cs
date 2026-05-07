using System;

namespace Neuro.Frontend
{
    public class ErrorReporter
    {
        public void ReportError(string message)
        {
            Console.WriteLine($"[Error] {message}");
        }
    }
}
