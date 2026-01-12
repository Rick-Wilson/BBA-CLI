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

        Console.WriteLine("=== Board 2 Test (All Conventions from 21GF-DEFAULT.bbsa) ===")
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

        ' TEST: Add conventions 7-12 to find which breaks it
        For k = 0 To 3
            Player(k).new_hand(k, hand(k).suit, dealer, vulnerable)
            Player(k).scoring = 0

            For side = 0 To 1
                Player(k).system_type(side) = T_21GF

                ' The minimal 3 that work
                Player(k).conventions(side, "Cue bid") = 1
                Player(k).conventions(side, "Cappelletti") = 1
                Player(k).conventions(side, "Lebensohl after 1NT") = 1

                ' FIRST QUARTER of additional conventions (conventions 1-6) - these work
                Player(k).conventions(side, "1m opening allows 5M") = 1
                Player(k).conventions(side, "1M-3M inviting") = 1
                Player(k).conventions(side, "1N-2S transfer to clubs") = 1
                Player(k).conventions(side, "1N-3C transfer to diamonds") = 1
                Player(k).conventions(side, "1N-3D natural") = 1
                Player(k).conventions(side, "1NT opening NT style") = 1

                ' Adding 7-12 - work fine
                Player(k).conventions(side, "1NT opening range 15-17") = 1
                Player(k).conventions(side, "1NT opening shape 5422") = 1
                Player(k).conventions(side, "1NT opening shape 5 major") = 1
                Player(k).conventions(side, "1X-(Y)-2Z forcing") = 1
                Player(k).conventions(side, "1X-(1Y)-2Z weak") = 1
                Player(k).conventions(side, "4NT opening") = 1

                ' Adding 13-18
                Player(k).conventions(side, "Blackwood 0314") = 1
                Player(k).conventions(side, "DOPI") = 1
                Player(k).conventions(side, "Extended acceptance after NT") = 1
                Player(k).conventions(side, "Forcing 1NT") = 1
                Player(k).conventions(side, "Fourth suit game force") = 1
                Player(k).conventions(side, "Gambling") = 1

                ' ALL conventions EXCEPT Garbage Stayman
                Player(k).conventions(side, "Gerber") = 1
                Player(k).conventions(side, "Inverted minors") = 1
                Player(k).conventions(side, "Jacoby 2NT") = 1
                Player(k).conventions(side, "Jordan Truscott 2NT") = 1
                Player(k).conventions(side, "King ask by 5NT") = 1
                Player(k).conventions(side, "Lavinthal from void") = 1
                Player(k).conventions(side, "Lavinthal on ace") = 1
                Player(k).conventions(side, "Lavinthal to void") = 1
                ' Second half of enabled conventions
                Player(k).conventions(side, "Lebensohl after 1m") = 1
                Player(k).conventions(side, "Lebensohl after double") = 1
                Player(k).conventions(side, "Mark on queen") = 1
                Player(k).conventions(side, "Mark on king") = 1
                Player(k).conventions(side, "Michaels Cuebid") = 1
                Player(k).conventions(side, "Minor Suit Transfers after 2NT") = 1
                Player(k).conventions(side, "New Minor Forcing") = 1
                Player(k).conventions(side, "Quantitative 4NT") = 1
                Player(k).conventions(side, "Responsive double") = 1
                Player(k).conventions(side, "Reverse drury") = 1
                Player(k).conventions(side, "ROPI") = 1
                Player(k).conventions(side, "Shape Bergen structure") = 1
                Player(k).conventions(side, "SMOLEN") = 1
                Player(k).conventions(side, "Splinter") = 1
                Player(k).conventions(side, "Strong jump shifts 2") = 1
                Player(k).conventions(side, "Super acceptance after NT") = 1
                Player(k).conventions(side, "Support double redouble") = 1
                Player(k).conventions(side, "Texas") = 1
                Player(k).conventions(side, "Transfers if RHO bids clubs") = 1
                Player(k).conventions(side, "Two suit takeout double") = 1
                Player(k).conventions(side, "Unusual 1NT") = 1
                Player(k).conventions(side, "Unusual 2NT") = 1
                Player(k).conventions(side, "Unusual 4NT") = 1
                Player(k).conventions(side, "Weak Jump Shifts 3") = 1
                Player(k).conventions(side, "Weak natural 2D") = 1
                Player(k).conventions(side, "Weak natural 2M") = 1

                Player(k).opponent_type(side) = 0
            Next side
        Next k

        Console.WriteLine("Conventions: ALL ENABLED except Garbage Stayman")
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
