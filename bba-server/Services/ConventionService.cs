using System.Text.RegularExpressions;
using BbaServer.Models;

namespace BbaServer.Services;

/// <summary>
/// Service for looking up convention cards from scenario .dlr files.
/// </summary>
public class ConventionService
{
    private readonly ILogger<ConventionService> _logger;
    private readonly string _dlrPath;
    private readonly string _bbsaPath;
    private readonly string _defaultNsCard;
    private readonly string _defaultEwCard;

    public ConventionService(ILogger<ConventionService> logger, IConfiguration configuration)
    {
        _logger = logger;

        // Read paths from configuration
        _dlrPath = configuration["Pbs:DlrPath"] ?? @"P:\dlr";
        _bbsaPath = configuration["Pbs:BbsaPath"] ?? @"P:\bbsa";
        _defaultNsCard = configuration["Pbs:DefaultNsCard"] ?? "21GF-DEFAULT";
        _defaultEwCard = configuration["Pbs:DefaultEwCard"] ?? "21GF-GIB";
    }

    /// <summary>
    /// Get convention cards for a scenario.
    /// </summary>
    /// <param name="scenario">Scenario name (e.g., "Smolen")</param>
    /// <returns>Convention cards for NS and EW</returns>
    public ConventionCards GetConventionsForScenario(string scenario)
    {
        var nsCard = GetConventionCardFromDlr(scenario) ?? _defaultNsCard;
        return new ConventionCards
        {
            Ns = nsCard,
            Ew = _defaultEwCard
        };
    }

    /// <summary>
    /// Get the full path to a convention card file.
    /// </summary>
    public string GetConventionFilePath(string cardName)
    {
        return Path.Combine(_bbsaPath, $"{cardName}.bbsa");
    }

    /// <summary>
    /// Read the convention-card property from a .dlr file.
    /// </summary>
    private string? GetConventionCardFromDlr(string scenario)
    {
        var dlrFile = Path.Combine(_dlrPath, $"{scenario}.dlr");

        if (!File.Exists(dlrFile))
        {
            _logger.LogWarning("DLR file not found: {DlrFile}", dlrFile);
            return null;
        }

        try
        {
            // Pattern to match: # convention-card: value
            var pattern = new Regex(@"^#?\s*convention-card:\s*(.+)$", RegexOptions.IgnoreCase);

            foreach (var line in File.ReadLines(dlrFile))
            {
                var match = pattern.Match(line.Trim());
                if (match.Success)
                {
                    var value = match.Groups[1].Value.Trim();
                    if (!string.IsNullOrEmpty(value))
                    {
                        _logger.LogInformation("Found convention card '{Card}' for scenario '{Scenario}'", value, scenario);
                        return value;
                    }
                }
            }
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error reading DLR file: {DlrFile}", dlrFile);
        }

        _logger.LogInformation("No convention card specified for scenario '{Scenario}', using default", scenario);
        return null;
    }

    /// <summary>
    /// Check if a scenario exists.
    /// </summary>
    public bool ScenarioExists(string scenario)
    {
        var dlrFile = Path.Combine(_dlrPath, $"{scenario}.dlr");
        return File.Exists(dlrFile);
    }

    /// <summary>
    /// Check if a convention card file exists.
    /// </summary>
    public bool ConventionFileExists(string cardName)
    {
        return File.Exists(GetConventionFilePath(cardName));
    }
}
