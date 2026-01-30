using BbaServer.Models;
using BbaServer.Services;

// Helper to format alerts for audit log
static string FormatAlerts(List<BidMeaning>? meanings)
{
    if (meanings == null) return "";
    var alerts = meanings
        .Where(m => m.IsAlert && !string.IsNullOrEmpty(m.Meaning))
        .Select(m => $"{m.Bid}={m.Meaning}");
    return string.Join("; ", alerts);
}

var builder = WebApplication.CreateBuilder(args);

// Add file logging (30-day retention)
var logDirectory = Path.Combine(AppContext.BaseDirectory, "logs");
builder.Logging.AddFileLogger(logDirectory, retentionDays: 30);

// Add services
builder.Services.AddSingleton<ConventionService>();
builder.Services.AddSingleton<EPBotService>();
builder.Services.AddSingleton<AuditLogService>();
builder.Services.AddSingleton<AdminService>();

// Add CORS
builder.Services.AddCors(options =>
{
    options.AddPolicy("AllowBBO", policy =>
    {
        policy.WithOrigins(
            "https://www.bridgebase.com",
            "http://www.bridgebase.com",
            "https://bridgebase.com",
            "http://localhost:3000" // For local development
        )
        .AllowAnyHeader()
        .AllowAnyMethod();
    });
});

var app = builder.Build();

app.UseCors("AllowBBO");

// Request logging middleware
app.Use(async (context, next) =>
{
    var logger = context.RequestServices.GetRequiredService<ILogger<Program>>();
    var path = context.Request.Path;
    var method = context.Request.Method;

    logger.LogInformation("Request: {Method} {Path}", method, path);

    var sw = System.Diagnostics.Stopwatch.StartNew();
    await next();
    sw.Stop();

    logger.LogInformation("Response: {Method} {Path} - {StatusCode} ({ElapsedMs}ms)",
        method, path, context.Response.StatusCode, sw.ElapsedMilliseconds);
});

// API Key validation middleware
app.Use(async (context, next) =>
{
    // Skip API key check for health endpoint, OpenAPI, and admin (admin has its own IP-based auth)
    if (context.Request.Path.StartsWithSegments("/health") ||
        context.Request.Path.StartsWithSegments("/openapi") ||
        context.Request.Path.StartsWithSegments("/admin"))
    {
        await next();
        return;
    }

    // Check for API key
    var configuredApiKey = app.Configuration["ApiKey"];
    if (!string.IsNullOrEmpty(configuredApiKey))
    {
        var providedApiKey = context.Request.Headers["X-API-Key"].FirstOrDefault()
            ?? context.Request.Query["apiKey"].FirstOrDefault();

        if (providedApiKey != configuredApiKey)
        {
            context.Response.StatusCode = 401;
            await context.Response.WriteAsJsonAsync(new AuctionResponse
            {
                Success = false,
                Error = "Invalid or missing API key"
            });
            return;
        }
    }

    await next();
});

// Health check endpoint
app.MapGet("/health", () => Results.Ok(new { status = "healthy", timestamp = DateTime.UtcNow }))
    .WithName("HealthCheck");

// Generate auction endpoint
app.MapPost("/api/auction/generate", async (
    HttpContext httpContext,
    AuctionRequest request,
    EPBotService epbotService,
    ConventionService conventionService,
    AuditLogService auditLog) =>
{
    var sw = System.Diagnostics.Stopwatch.StartNew();

    // Get client IP - check Cloudflare/proxy headers first, then fall back to connection IP
    var rawIP = httpContext.Request.Headers["CF-Connecting-IP"].FirstOrDefault()
        ?? httpContext.Request.Headers["X-Forwarded-For"].FirstOrDefault()?.Split(',').FirstOrDefault()?.Trim()
        ?? httpContext.Connection.RemoteIpAddress?.ToString();

    // Anonymize IP for privacy (generates friendly name like "Alice_Baker")
    var requestIP = IpAnonymizer.Anonymize(rawIP);

    // Get client version from header
    var clientVersion = httpContext.Request.Headers["X-Client-Version"].FirstOrDefault();

    // Determine convention cards
    ConventionCards conventions;
    var logger = httpContext.RequestServices.GetRequiredService<ILogger<Program>>();

    if (!string.IsNullOrEmpty(request.Scenario))
    {
        // Look up conventions from scenario (fall back to defaults if not found)
        if (conventionService.ScenarioExists(request.Scenario))
        {
            conventions = conventionService.GetConventionsForScenario(request.Scenario);
        }
        else
        {
            logger.LogWarning("Scenario not found: {Scenario}, using default conventions", request.Scenario);
            conventions = new ConventionCards();
        }
    }
    else if (request.Conventions != null)
    {
        // Use explicit conventions
        conventions = request.Conventions;
    }
    else
    {
        // Use defaults
        conventions = new ConventionCards();
    }

    // Validate convention files exist
    if (!conventionService.ConventionFileExists(conventions.Ns))
    {
        sw.Stop();
        var errorResponse = new AuctionResponse
        {
            Success = false,
            Error = $"NS convention card not found: {conventions.Ns}"
        };
        auditLog.LogRequest(requestIP, clientVersion, sw.ElapsedMilliseconds, epbotService.EPBotVersion,
            request.Deal.Dealer, request.Deal.Vulnerability, request.Deal.Scoring,
            conventions.Ns, conventions.Ew, request.Scenario, request.Deal.Pbn,
            false, null, null, errorResponse.Error);
        return Results.BadRequest(errorResponse);
    }
    if (!conventionService.ConventionFileExists(conventions.Ew))
    {
        sw.Stop();
        var errorResponse = new AuctionResponse
        {
            Success = false,
            Error = $"EW convention card not found: {conventions.Ew}"
        };
        auditLog.LogRequest(requestIP, clientVersion, sw.ElapsedMilliseconds, epbotService.EPBotVersion,
            request.Deal.Dealer, request.Deal.Vulnerability, request.Deal.Scoring,
            conventions.Ns, conventions.Ew, request.Scenario, request.Deal.Pbn,
            false, null, null, errorResponse.Error);
        return Results.BadRequest(errorResponse);
    }

    // Generate auction
    var response = await epbotService.GenerateAuctionAsync(request.Deal, conventions);
    sw.Stop();

    // Log to audit CSV
    auditLog.LogRequest(requestIP, clientVersion, sw.ElapsedMilliseconds, epbotService.EPBotVersion,
        request.Deal.Dealer, request.Deal.Vulnerability, request.Deal.Scoring,
        conventions.Ns, conventions.Ew, request.Scenario, request.Deal.Pbn,
        response.Success, response.AuctionEncoded, FormatAlerts(response.Meanings), response.Error);

    return Results.Ok(response);
})
.WithName("GenerateAuction");

// Record scenario selection endpoint
app.MapPost("/api/scenario/select", (
    HttpContext httpContext,
    ScenarioSelectRequest request,
    AuditLogService auditLog) =>
{
    // Get client IP - check Cloudflare/proxy headers first
    var rawIP = httpContext.Request.Headers["CF-Connecting-IP"].FirstOrDefault()
        ?? httpContext.Request.Headers["X-Forwarded-For"].FirstOrDefault()?.Split(',').FirstOrDefault()?.Trim()
        ?? httpContext.Connection.RemoteIpAddress?.ToString();

    var requestIP = IpAnonymizer.Anonymize(rawIP);
    var clientVersion = httpContext.Request.Headers["X-Client-Version"].FirstOrDefault();

    // Log the scenario selection
    auditLog.LogScenarioSelection(requestIP, clientVersion, request.Scenario ?? "");

    return Results.Ok(new { success = true });
})
.WithName("SelectScenario");

// List available scenarios endpoint
app.MapGet("/api/scenarios", (ConventionService conventionService, IConfiguration config) =>
{
    var dlrPath = config["Pbs:DlrPath"] ?? @"P:\dlr";
    if (!Directory.Exists(dlrPath))
    {
        return Results.Ok(new { scenarios = Array.Empty<string>(), error = "DLR directory not found" });
    }

    var scenarios = Directory.GetFiles(dlrPath, "*.dlr")
        .Select(f => Path.GetFileNameWithoutExtension(f))
        .OrderBy(s => s)
        .ToList();

    return Results.Ok(new { scenarios });
})
.WithName("ListScenarios");

// Helper to check admin access
static string GetAnonIP(HttpContext ctx)
{
    var rawIP = ctx.Request.Headers["CF-Connecting-IP"].FirstOrDefault()
        ?? ctx.Request.Headers["X-Forwarded-For"].FirstOrDefault()?.Split(',').FirstOrDefault()?.Trim()
        ?? ctx.Connection.RemoteIpAddress?.ToString();
    return IpAnonymizer.Anonymize(rawIP);
}

// Admin dashboard (HTML page)
app.MapGet("/admin", (HttpContext ctx, AdminService admin) =>
{
    var anonIP = GetAnonIP(ctx);
    if (!admin.IsAllowed(anonIP))
    {
        return Results.Unauthorized();
    }
    return Results.Redirect("/admin/dashboard");
})
.WithName("AdminRoot")
.ExcludeFromDescription();

app.MapGet("/admin/dashboard", (HttpContext ctx, AdminService admin) =>
{
    var anonIP = GetAnonIP(ctx);
    if (!admin.IsAllowed(anonIP))
    {
        return Results.Unauthorized();
    }
    return Results.Content(AdminDashboard.GetHtml(), "text/html");
})
.WithName("AdminDashboard")
.ExcludeFromDescription();

// Admin API: List log files
app.MapGet("/admin/api/logs", (HttpContext ctx, AdminService admin) =>
{
    var anonIP = GetAnonIP(ctx);
    if (!admin.IsAllowed(anonIP))
    {
        return Results.Unauthorized();
    }
    return Results.Ok(admin.GetLogFiles());
})
.WithName("AdminListLogs");

// Admin API: Get log file content
app.MapGet("/admin/api/logs/{filename}", (HttpContext ctx, string filename, AdminService admin) =>
{
    var anonIP = GetAnonIP(ctx);
    if (!admin.IsAllowed(anonIP))
    {
        return Results.Unauthorized();
    }

    var content = admin.GetLogContent(filename);
    if (content == null)
    {
        return Results.NotFound();
    }

    // Return as text for .log files, JSON for CSV
    if (filename.EndsWith(".csv"))
    {
        var data = admin.GetAuctionLogData(filename);
        return Results.Ok(new { filename, rowCount = data.Count, data });
    }

    return Results.Ok(new { filename, content });
})
.WithName("AdminGetLog");

// Admin API: Get statistics
app.MapGet("/admin/api/stats", (HttpContext ctx, AdminService admin) =>
{
    var anonIP = GetAnonIP(ctx);
    if (!admin.IsAllowed(anonIP))
    {
        return Results.Unauthorized();
    }
    return Results.Ok(admin.GetStats());
})
.WithName("AdminStats");

app.Run();

/// <summary>
/// Embedded HTML dashboard for admin interface.
/// </summary>
static class AdminDashboard
{
    public static string GetHtml() => """
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>BBA Server Admin</title>
    <style>
        * { box-sizing: border-box; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            margin: 0; padding: 20px;
            background: #f5f5f5;
        }
        h1 { color: #333; margin-bottom: 20px; }
        h2 { color: #555; margin-top: 30px; border-bottom: 2px solid #ddd; padding-bottom: 10px; }
        .stats-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 20px;
            margin-bottom: 30px;
        }
        .stat-card {
            background: white;
            padding: 20px;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }
        .stat-card h3 { margin: 0 0 10px 0; color: #666; font-size: 14px; }
        .stat-card .value { font-size: 32px; font-weight: bold; color: #333; }
        .stat-card.success .value { color: #28a745; }
        .stat-card.error .value { color: #dc3545; }
        table {
            width: 100%;
            border-collapse: collapse;
            background: white;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
            border-radius: 8px;
            overflow: hidden;
        }
        th, td {
            padding: 12px;
            text-align: left;
            border-bottom: 1px solid #eee;
        }
        th { background: #f8f9fa; font-weight: 600; color: #555; }
        tr:hover { background: #f8f9fa; }
        .file-link { color: #007bff; cursor: pointer; text-decoration: none; }
        .file-link:hover { text-decoration: underline; }
        .tabs {
            display: flex;
            gap: 10px;
            margin-bottom: 20px;
        }
        .tab {
            padding: 10px 20px;
            background: white;
            border: none;
            border-radius: 8px 8px 0 0;
            cursor: pointer;
            font-size: 14px;
        }
        .tab.active { background: #007bff; color: white; }
        .tab-content { display: none; }
        .tab-content.active { display: block; }
        .log-viewer {
            background: #1e1e1e;
            color: #d4d4d4;
            padding: 20px;
            border-radius: 8px;
            font-family: 'Monaco', 'Menlo', monospace;
            font-size: 12px;
            max-height: 500px;
            overflow: auto;
            white-space: pre-wrap;
        }
        .error-row { background: #fff5f5; }
        .refresh-btn {
            background: #007bff;
            color: white;
            border: none;
            padding: 10px 20px;
            border-radius: 4px;
            cursor: pointer;
            margin-bottom: 20px;
        }
        .refresh-btn:hover { background: #0056b3; }
        .chart-container { height: 200px; margin: 20px 0; }
        .bar-chart {
            display: flex;
            align-items: flex-end;
            height: 150px;
            gap: 4px;
            padding: 10px;
            background: white;
            border-radius: 8px;
        }
        .bar {
            flex: 1;
            background: #007bff;
            min-width: 20px;
            position: relative;
        }
        .bar-label {
            position: absolute;
            bottom: -25px;
            left: 50%;
            transform: translateX(-50%);
            font-size: 10px;
            color: #666;
            white-space: nowrap;
        }
        .bar-value {
            position: absolute;
            top: -20px;
            left: 50%;
            transform: translateX(-50%);
            font-size: 11px;
            font-weight: bold;
        }
        .legend { display: flex; gap: 20px; margin: 10px 0; flex-wrap: wrap; }
        .legend-item { display: flex; align-items: center; gap: 5px; font-size: 12px; }
        .legend-color { width: 12px; height: 12px; border-radius: 2px; }
    </style>
</head>
<body>
    <h1>🎴 BBA Server Admin Dashboard</h1>

    <button class="refresh-btn" onclick="loadAll()">↻ Refresh</button>

    <div class="stats-grid" id="stats-grid">
        <div class="stat-card"><h3>Loading...</h3><div class="value">-</div></div>
    </div>

    <div class="tabs">
        <button class="tab active" onclick="showTab('overview')">Overview</button>
        <button class="tab" onclick="showTab('logs')">Log Files</button>
        <button class="tab" onclick="showTab('requests')">Recent Requests</button>
        <button class="tab" onclick="showTab('errors')">Errors</button>
    </div>

    <div id="overview" class="tab-content active">
        <h2>Requests by Day</h2>
        <div class="bar-chart" id="daily-chart"></div>

        <h2>Requests by Scenario</h2>
        <div id="scenario-table"></div>

        <h2>Requests by User</h2>
        <div id="user-table"></div>
    </div>

    <div id="logs" class="tab-content">
        <h2>Log Files</h2>
        <table id="files-table">
            <thead><tr><th>Name</th><th>Size</th><th>Modified</th><th>Type</th></tr></thead>
            <tbody></tbody>
        </table>
    </div>

    <div id="requests" class="tab-content">
        <h2>Recent Auction Requests</h2>
        <table id="requests-table">
            <thead><tr><th>Time</th><th>User</th><th>Scenario</th><th>Auction</th><th>Duration</th><th>Status</th></tr></thead>
            <tbody></tbody>
        </table>
    </div>

    <div id="errors" class="tab-content">
        <h2>Recent Errors</h2>
        <table id="errors-table">
            <thead><tr><th>Time</th><th>Scenario</th><th>Error</th></tr></thead>
            <tbody></tbody>
        </table>
    </div>

    <script>
        let currentStats = null;
        let currentLogs = null;
        let currentAuctionData = null;

        async function loadAll() {
            await Promise.all([loadStats(), loadFiles(), loadAuctionLog()]);
        }

        async function loadStats() {
            try {
                const res = await fetch('/admin/api/stats');
                if (!res.ok) throw new Error('Unauthorized');
                currentStats = await res.json();
                renderStats();
                renderCharts();
            } catch (e) {
                document.getElementById('stats-grid').innerHTML = '<div class="stat-card error"><h3>Error</h3><div class="value">' + e.message + '</div></div>';
            }
        }

        async function loadFiles() {
            try {
                const res = await fetch('/admin/api/logs');
                if (!res.ok) throw new Error('Unauthorized');
                currentLogs = await res.json();
                renderFiles();
            } catch (e) {
                console.error(e);
            }
        }

        async function loadAuctionLog() {
            try {
                const res = await fetch('/admin/api/logs');
                if (!res.ok) return;
                const files = await res.json();
                const auctionLog = files.find(f => f.name.startsWith('audit-auction-'));
                if (!auctionLog) return;

                const logRes = await fetch('/admin/api/logs/' + auctionLog.name);
                if (!logRes.ok) return;
                const data = await logRes.json();
                currentAuctionData = data.data || [];
                renderRequests();
            } catch (e) {
                console.error(e);
            }
        }

        function renderStats() {
            if (!currentStats) return;
            const s = currentStats;
            document.getElementById('stats-grid').innerHTML = `
                <div class="stat-card"><h3>Total Requests</h3><div class="value">${s.totalRequests}</div></div>
                <div class="stat-card success"><h3>Successful</h3><div class="value">${s.successfulRequests}</div></div>
                <div class="stat-card error"><h3>Failed</h3><div class="value">${s.failedRequests}</div></div>
                <div class="stat-card"><h3>Avg Duration</h3><div class="value">${s.averageDurationMs}ms</div></div>
                <div class="stat-card"><h3>Max Duration</h3><div class="value">${s.maxDurationMs}ms</div></div>
                <div class="stat-card"><h3>Unique Users</h3><div class="value">${Object.keys(s.requestsByUser || {}).length}</div></div>
            `;
        }

        function renderCharts() {
            if (!currentStats) return;

            // Daily chart
            const daily = currentStats.requestsByDay || {};
            const days = Object.entries(daily).slice(-14);
            const maxDaily = Math.max(...days.map(d => d[1]), 1);

            document.getElementById('daily-chart').innerHTML = days.map(([day, count]) => `
                <div class="bar" style="height: ${(count / maxDaily) * 100}%">
                    <span class="bar-value">${count}</span>
                    <span class="bar-label">${day.slice(5)}</span>
                </div>
            `).join('');

            // Scenario table
            const scenarios = Object.entries(currentStats.requestsByScenario || {}).sort((a, b) => b[1] - a[1]);
            document.getElementById('scenario-table').innerHTML = `<table><thead><tr><th>Scenario</th><th>Count</th></tr></thead><tbody>
                ${scenarios.map(([s, c]) => `<tr><td>${s || '(none)'}</td><td>${c}</td></tr>`).join('')}
            </tbody></table>`;

            // User table
            const users = Object.entries(currentStats.requestsByUser || {}).sort((a, b) => b[1] - a[1]);
            document.getElementById('user-table').innerHTML = `<table><thead><tr><th>User</th><th>Count</th></tr></thead><tbody>
                ${users.map(([u, c]) => `<tr><td>${u}</td><td>${c}</td></tr>`).join('')}
            </tbody></table>`;

            // Errors table
            const errors = currentStats.recentErrors || [];
            document.getElementById('errors-table').querySelector('tbody').innerHTML = errors.map(e => `
                <tr class="error-row"><td>${e.timestamp}</td><td>${e.scenario || '-'}</td><td>${e.error}</td></tr>
            `).join('') || '<tr><td colspan="3">No recent errors</td></tr>';
        }

        function renderFiles() {
            if (!currentLogs) return;
            const tbody = document.querySelector('#files-table tbody');
            tbody.innerHTML = currentLogs.map(f => `
                <tr>
                    <td><a class="file-link" onclick="viewFile('${f.name}')">${f.name}</a></td>
                    <td>${formatBytes(f.size)}</td>
                    <td>${new Date(f.lastModified).toLocaleString()}</td>
                    <td>${f.type}</td>
                </tr>
            `).join('');
        }

        function renderRequests() {
            if (!currentAuctionData) return;
            const tbody = document.querySelector('#requests-table tbody');
            const recent = currentAuctionData.slice(-100).reverse();
            tbody.innerHTML = recent.map(r => `
                <tr class="${r.Success === 'false' ? 'error-row' : ''}">
                    <td>${r.Timestamp || '-'}</td>
                    <td>${r.RequestIP || '-'}</td>
                    <td>${r.Scenario || '-'}</td>
                    <td>${r.Auction || '-'}</td>
                    <td>${r.DurationMs || '-'}ms</td>
                    <td>${r.Success === 'true' ? '✓' : '✗'}</td>
                </tr>
            `).join('');
        }

        async function viewFile(filename) {
            try {
                const res = await fetch('/admin/api/logs/' + filename);
                if (!res.ok) throw new Error('Failed to load file');
                const data = await res.json();

                if (data.data) {
                    // CSV file - show as table
                    const headers = data.data.length > 0 ? Object.keys(data.data[0]) : [];
                    alert('CSV file with ' + data.rowCount + ' rows. View in Recent Requests tab for formatted view.');
                } else {
                    // Log file - show content
                    alert(data.content?.slice(0, 5000) || 'Empty file');
                }
            } catch (e) {
                alert('Error: ' + e.message);
            }
        }

        function showTab(name) {
            document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
            document.querySelectorAll('.tab-content').forEach(t => t.classList.remove('active'));
            document.querySelector(`.tab-content#${name}`).classList.add('active');
            event.target.classList.add('active');
        }

        function formatBytes(bytes) {
            if (bytes < 1024) return bytes + ' B';
            if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
            return (bytes / 1024 / 1024).toFixed(1) + ' MB';
        }

        loadAll();
        setInterval(loadAll, 60000); // Refresh every minute
    </script>
</body>
</html>
""";
}
