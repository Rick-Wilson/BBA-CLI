Module ModuleBidding
    Public current_double As Integer, current_bid As Integer, leader As Integer, dummy As Integer, declarer As Integer, initial_pauses As Integer
    Public arr_bids() As String
    Public Sub bid()
        Dim meaning As String
        Dim k As Integer, passes As Integer, position As Integer, bid_number As Integer, new_bid As Integer, side As Integer, strain As Integer
        Dim alerting As Boolean
        Dim declarers(1, 4) As Integer
        position = dealer
        initial_pauses = (dealer + 1) Mod 4
        For bid_number = 0 To UBound(arr_bids)
            With Player(position)
                new_bid = .get_bid()
                For k = 0 To 3
                    '---send new_bid to all players
                    Player(k).set_bid(position, new_bid) ', alert
                Next k
                alerting = .info_alerting(position)
                meaning = .info_meaning(position)
                arr_bids(bid_number) = Format$(new_bid, "00")
                If alerting Then
                    '---alerts
                    arr_bids(bid_number) = arr_bids(bid_number) & "*" & meaning
                End If

                ''Console.WriteLine(CStr(bid_number) & "-" & arr_bids(bid_number))

                If new_bid = 0 Then
                    passes = passes + 1
                    If passes = 4 Or passes = 3 And current_bid > 0 Then
                        declarer = (declarers(side, strain) + 3) Mod 4
                        leader = (declarer + 1) Mod 4
                        dummy = (declarer + 2) Mod 4
                        last_bid = bid_number
                        Exit For
                    End If
                Else
                    passes = 0
                    If new_bid <= 2 Then
                        current_double = new_bid
                    Else
                        current_bid = new_bid
                        current_double = 0
                        side = position Mod 2
                        strain = current_bid Mod C_FIVE
                        If declarers(side, strain) = 0 Then
                            declarers(side, strain) = position + 1
                        End If
                    End If
                End If
            End With
            position = (position + 1) Mod 4
        Next bid_number
    End Sub
End Module
