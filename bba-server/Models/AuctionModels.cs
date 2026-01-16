namespace BbaServer.Models;

/// <summary>
/// Request to generate an auction for a deal.
/// </summary>
public class AuctionRequest
{
    /// <summary>
    /// The deal information.
    /// </summary>
    public required DealInfo Deal { get; set; }

    /// <summary>
    /// Optional scenario name (e.g., "Smolen").
    /// If provided, convention cards are looked up from the .dlr file.
    /// </summary>
    public string? Scenario { get; set; }

    /// <summary>
    /// Optional explicit convention card specifications.
    /// Used if Scenario is not provided.
    /// </summary>
    public ConventionCards? Conventions { get; set; }
}

/// <summary>
/// Deal information in PBN format.
/// </summary>
public class DealInfo
{
    /// <summary>
    /// PBN deal string, e.g., "N:A653.Q97.K64.954 KQ4.AT8432.A72.A J8.K.QJ98.KJT762 T972.J65.T53.Q83"
    /// </summary>
    public required string Pbn { get; set; }

    /// <summary>
    /// Dealer position: N, E, S, or W
    /// </summary>
    public required string Dealer { get; set; }

    /// <summary>
    /// Vulnerability: None, NS, EW, or Both
    /// </summary>
    public required string Vulnerability { get; set; }

    /// <summary>
    /// Scoring method: MP (Matchpoints) or IMP. Defaults to MP.
    /// </summary>
    public string Scoring { get; set; } = "MP";
}

/// <summary>
/// Convention card specifications.
/// </summary>
public class ConventionCards
{
    /// <summary>
    /// North-South convention card name (without .bbsa extension)
    /// </summary>
    public string Ns { get; set; } = "21GF-DEFAULT";

    /// <summary>
    /// East-West convention card name (without .bbsa extension)
    /// </summary>
    public string Ew { get; set; } = "21GF-GIB";
}

/// <summary>
/// Response from auction generation.
/// </summary>
public class AuctionResponse
{
    /// <summary>
    /// Whether the request was successful.
    /// </summary>
    public bool Success { get; set; }

    /// <summary>
    /// The generated auction as a list of bids (e.g., ["1C", "Pass", "1S", ...])
    /// </summary>
    public List<string>? Auction { get; set; }

    /// <summary>
    /// The auction encoded in BBOAlert format (e.g., "1C--1S2H3S--4S------")
    /// </summary>
    public string? AuctionEncoded { get; set; }

    /// <summary>
    /// The convention cards that were used.
    /// </summary>
    public ConventionCards? ConventionsUsed { get; set; }

    /// <summary>
    /// Bid meanings from EPBot.
    /// </summary>
    public List<BidMeaning>? Meanings { get; set; }

    /// <summary>
    /// Error message if Success is false.
    /// </summary>
    public string? Error { get; set; }
}

/// <summary>
/// Meaning of a bid from EPBot.
/// </summary>
public class BidMeaning
{
    /// <summary>
    /// Position in the auction (0-indexed).
    /// </summary>
    public int Position { get; set; }

    /// <summary>
    /// The bid (e.g., "1C", "Pass", "X").
    /// </summary>
    public required string Bid { get; set; }

    /// <summary>
    /// The meaning/explanation of the bid.
    /// </summary>
    public string? Meaning { get; set; }
}
