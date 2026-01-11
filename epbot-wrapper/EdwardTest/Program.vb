Imports System

Module Program
    Sub Main(args As String())
        Dim k As Integer
        ReDim hand(3)
        ReDim arr_bids(63)

        For k = 0 To 3
            ReDim hand(k).suit(3)
        Next
        For k = 0 To 3
            Player(k) = New EPBot64.EPBot
        Next k

        set_board()
        set_dealers()
        set_vulnerability()
        set_strain_mark()

        ' Board 2 from reference PBN - hex code includes dealer/vulnerability
        Dim str_hand As String = "0AA3C6EC2DD39E66314C111542D2"

        Console.WriteLine("=== Board 2 Test (Minimal Conventions) ===")
        Console.WriteLine("Hex: " & str_hand)
        Console.WriteLine("Expected: 1NT 2C X Pass 2H Pass 3NT Pass Pass Pass")
        Console.WriteLine()

        ' Decode the hand using Edward's set_hand function
        set_hand(str_hand)

        ' Display decoded values
        Console.WriteLine("Deal = " & CStr(deal))
        Console.WriteLine("Dealer = " & CStr(dealer))
        Console.WriteLine("Vulnerable = " & CStr(vulnerable))
        Console.WriteLine()

        ' Display hands
        Console.WriteLine("        North")
        For i = C_SPADES To C_CLUBS Step -1
            Console.WriteLine("        " + hand(0).suit(i))
        Next i
        Console.WriteLine()
        Console.WriteLine("West            East")
        For i = C_SPADES To C_CLUBS Step -1
            Dim w As String = If(hand(3).suit(i), "")
            Dim e As String = If(hand(1).suit(i), "")
            Console.WriteLine(w.PadRight(8) + "        " + e)
        Next i
        Console.WriteLine()
        Console.WriteLine("        South")
        For i = C_SPADES To C_CLUBS Step -1
            Console.WriteLine("        " + hand(2).suit(i))
        Next i
        Console.WriteLine()

        ' Edward's setup + Cappelletti + Lebensohl
        For k = 0 To 3
            Player(k).new_hand(k, hand(k).suit, dealer, vulnerable)
            Player(k).scoring = 0
            ' Team NS (side 0)
            Player(k).system_type(0) = T_21GF
            Player(k).conventions(0, "Cue bid") = 1
            Player(k).conventions(0, "Cappelletti") = 1
            Player(k).conventions(0, "Lebensohl after 1NT") = 1
            ' Team EW (side 1)
            Player(k).system_type(1) = T_21GF
            Player(k).conventions(1, "Cue bid") = 1
            Player(k).conventions(1, "Cappelletti") = 1
            Player(k).conventions(1, "Lebensohl after 1NT") = 1
        Next k

        Console.WriteLine("Conventions: system_type=T_21GF, Cue bid=1, Cappelletti=1, Lebensohl after 1NT=1")
        Console.WriteLine()

        ' Run bidding
        bid()

        Console.WriteLine("W" + vbTab + "N" + vbTab + "E" + vbTab + "S" + vbTab)
        Console.WriteLine(bidding_body)
        Console.WriteLine()
        Console.WriteLine("Declarer = " & CStr(declarer))
        Console.WriteLine("Leader = " & CStr(leader))
        Console.WriteLine("Dummy = " & CStr(dummy))
    End Sub
End Module
