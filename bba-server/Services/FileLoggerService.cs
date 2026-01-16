namespace BbaServer.Services;

/// <summary>
/// Simple file logger that writes to daily log files with monthly retention.
/// </summary>
public class FileLoggerProvider : ILoggerProvider
{
    private readonly string _logDirectory;
    private readonly int _retentionDays;

    public FileLoggerProvider(string logDirectory, int retentionDays = 30)
    {
        _logDirectory = logDirectory;
        _retentionDays = retentionDays;
        Directory.CreateDirectory(_logDirectory);
        CleanupOldLogs();
    }

    public ILogger CreateLogger(string categoryName)
    {
        return new FileLogger(_logDirectory, categoryName);
    }

    public void Dispose()
    {
    }

    private void CleanupOldLogs()
    {
        try
        {
            var cutoff = DateTime.Now.AddDays(-_retentionDays);
            foreach (var file in Directory.GetFiles(_logDirectory, "*.log"))
            {
                if (File.GetLastWriteTime(file) < cutoff)
                {
                    File.Delete(file);
                }
            }
        }
        catch
        {
            // Ignore cleanup errors
        }
    }
}

public class FileLogger : ILogger
{
    private readonly string _logDirectory;
    private readonly string _categoryName;
    private static readonly object _lock = new();

    public FileLogger(string logDirectory, string categoryName)
    {
        _logDirectory = logDirectory;
        _categoryName = categoryName;
    }

    public IDisposable? BeginScope<TState>(TState state) where TState : notnull => null;

    public bool IsEnabled(LogLevel logLevel) => logLevel >= LogLevel.Information;

    public void Log<TState>(LogLevel logLevel, EventId eventId, TState state, Exception? exception, Func<TState, Exception?, string> formatter)
    {
        if (!IsEnabled(logLevel)) return;

        var logFile = Path.Combine(_logDirectory, $"bba-server-{DateTime.Now:yyyy-MM-dd}.log");
        var timestamp = DateTime.Now.ToString("yyyy-MM-dd HH:mm:ss.fff");
        var level = logLevel.ToString().ToUpper().PadRight(11);
        var category = _categoryName.Length > 40 ? _categoryName[^40..] : _categoryName;
        var message = formatter(state, exception);

        var logLine = $"{timestamp} [{level}] {category}: {message}";
        if (exception != null)
        {
            logLine += Environment.NewLine + exception.ToString();
        }

        lock (_lock)
        {
            try
            {
                File.AppendAllText(logFile, logLine + Environment.NewLine);
            }
            catch
            {
                // Ignore write errors
            }
        }
    }
}

public static class FileLoggerExtensions
{
    public static ILoggingBuilder AddFileLogger(this ILoggingBuilder builder, string logDirectory, int retentionDays = 30)
    {
        builder.AddProvider(new FileLoggerProvider(logDirectory, retentionDays));
        return builder;
    }
}
