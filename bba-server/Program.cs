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
    // Skip API key check for health endpoint and OpenAPI
    if (context.Request.Path.StartsWithSegments("/health") ||
        context.Request.Path.StartsWithSegments("/openapi"))
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

    if (!string.IsNullOrEmpty(request.Scenario))
    {
        // Look up conventions from scenario
        if (!conventionService.ScenarioExists(request.Scenario))
        {
            sw.Stop();
            var errorResponse = new AuctionResponse
            {
                Success = false,
                Error = $"Scenario not found: {request.Scenario}"
            };
            auditLog.LogRequest(requestIP, clientVersion, sw.ElapsedMilliseconds, epbotService.EPBotVersion,
                request.Deal.Dealer, request.Deal.Vulnerability, request.Deal.Scoring,
                "", "", request.Scenario, request.Deal.Pbn,
                false, null, null, errorResponse.Error);
            return Results.BadRequest(errorResponse);
        }
        conventions = conventionService.GetConventionsForScenario(request.Scenario);
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

app.Run();
