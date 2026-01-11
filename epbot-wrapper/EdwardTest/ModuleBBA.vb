Imports System.Runtime.CompilerServices

Module ModuleBBA
    Private i As Integer, j As Integer, k As Integer, number As Integer, encryption_byte As Integer, board_extension As Integer
    Private str_card As String, str_number As String
    Public Const C_PASS As Integer = 0
    Public Const C_DOUBLE As Integer = 1
    Public Const C_REDOUBLE As Integer = 2
    Public Const C_CLUBS As Integer = 0
    Public Const C_DIAMONDS As Integer = 1
    Public Const C_HEARTS As Integer = 2
    Public Const C_SPADES As Integer = 3
    Public Const C_NT As Integer = 4
    Public Const C_NORTH As Integer = 0
    Public Const C_EAST As Integer = 1
    Public Const C_SOUTH As Integer = 2
    Public Const C_WEST As Integer = 3
    Public Const V_NONE As Integer = 0
    Public Const V_NS As Integer = 2
    Public Const V_WE As Integer = 1
    Public Const V_BOTH As Integer = 3
    Public Const T_21GF As Integer = 0
    Public Const T_SAYC As Integer = 1
    Public Const T_WJ As Integer = 2
    Public Const T_PC As Integer = 3
    Public Const T_ACOL As Integer = 4
    Public Const C_FIVE As Integer = 5
    Public Const C_LONGER As String = "AKQJT98765432"
    Public Const C_INTERPRETED As Integer = 13
    Public Const F_MIN_HCP As Integer = 102
    Public Const F_MAX_HCP As Integer = 103
    Public Const F_MIN_PKT As Integer = 104
    Public Const F_MAX_PKT As Integer = 105
    Public deal As Integer, dealer As Integer, vulnerable As Integer, last_bid As Integer
    Public board(0 To 3, 0 To 3) As Integer
    Public dealers(0 To 15) As Integer
    Public vulnerability(15) As Integer
    Public strain_mark(5) As String
    Private lbloki(3) As Integer
    ''EPBot is a namespace - using 64-bit version
    'Public Player(3) As EPBot86.EPBot
    Public Player(3) As EPBot64.EPBot
    ''Dim Player(3) As EPBotARM64.EPBot
    Public Structure TYPE_HAND
        Dim suit() As String
    End Structure
    Public hand() As TYPE_HAND
    Public Sub set_board()
        '---standard number of a board
        board(C_NORTH, V_NONE) = 1
        board(C_EAST, V_NS) = 2
        board(C_SOUTH, V_WE) = 3
        board(C_WEST, V_BOTH) = 4
        board(C_NORTH, V_NS) = 5
        board(C_EAST, V_WE) = 6
        board(C_SOUTH, V_BOTH) = 7
        board(C_WEST, V_NONE) = 8
        board(C_NORTH, V_WE) = 9
        board(C_EAST, V_BOTH) = 10
        board(C_SOUTH, V_NONE) = 11
        board(C_WEST, V_NS) = 12
        board(C_NORTH, V_BOTH) = 13
        board(C_EAST, V_NONE) = 14
        board(C_SOUTH, V_NS) = 15
        board(C_WEST, V_WE) = 16
    End Sub
    Public Sub set_dealers()
        Dim k As Integer
        For k = 0 To 12 Step 4
            dealers(0 + k) = C_NORTH
            dealers(1 + k) = C_EAST
            dealers(2 + k) = C_SOUTH
            dealers(3 + k) = C_WEST
        Next k
    End Sub
    Public Sub set_vulnerability()
        vulnerability(0) = V_NONE
        vulnerability(1) = V_NS
        vulnerability(2) = V_WE
        vulnerability(3) = V_BOTH
        vulnerability(4) = V_NS
        vulnerability(5) = V_WE
        vulnerability(6) = V_BOTH
        vulnerability(7) = V_NONE
        vulnerability(8) = V_WE
        vulnerability(9) = V_BOTH
        vulnerability(10) = V_NONE
        vulnerability(11) = V_NS
        vulnerability(12) = V_BOTH
        vulnerability(13) = V_NONE
        vulnerability(14) = V_NS
        vulnerability(15) = V_WE
    End Sub
    Public Sub set_strain_mark()
        strain_mark(C_CLUBS) = "C"
        strain_mark(C_DIAMONDS) = "D"
        strain_mark(C_HEARTS) = "H"
        strain_mark(C_SPADES) = "S"
        strain_mark(C_NT) = "N"
        strain_mark(5) = ""
    End Sub
    Public Function bid_name(ByVal a_bid As Integer) As String
        If a_bid < 0 Then
            bid_name = vbNullString
        ElseIf a_bid = 0 Then
            bid_name = "P"
        ElseIf a_bid = 1 Then
            bid_name = "X"
        ElseIf a_bid = 2 Then
            bid_name = "XX"
        Else
            bid_name = CStr(a_bid \ C_FIVE) & strain_mark(a_bid Mod C_FIVE)
        End If
    End Function

    Public Sub set_hand(BBA_NUMBER As String)
        str_number = Left$(BBA_NUMBER, 1)
        board_extension = CLng("&H" & str_number)
        str_number = Mid$(BBA_NUMBER, 2, 1)
        number = CLng("&H" & str_number)
        dealer = number \ 4
        vulnerable = number Mod 4
        deal = board_extension * 16 + board(dealer, vulnerable)
        encryption_byte = board(dealer, vulnerable)
        For j = 1 To 13
            str_card = Mid$(C_LONGER, j, 1)
            str_number = Mid$(BBA_NUMBER, 2 * j + 1, 2)
            '---0-15
            number = CLng("&H" & str_number)
            number = encryption_byte Xor number
            lbloki(0) = number Mod 4
            lbloki(1) = (number \ 4) Mod 4
            lbloki(2) = (number \ 16) Mod 4
            lbloki(3) = number \ 64
            For i = 0 To 3
                k = lbloki(i)
                hand(k).suit(i) = hand(k).suit(i) & str_card
            Next i
        Next j
    End Sub
    Public Function bidding_body() As String
        Dim int_bid As Integer, lp As Integer, note_index As Integer
        Dim str_bid As String, str_bidding As String, note_text As String
        note_text = vbNullString
        str_bidding = vbNullString
        lp = initial_pauses
        If initial_pauses > 0 Then
            str_bidding = StrDup(initial_pauses, vbTab)
        End If
        For j = 0 To last_bid
            '---bidding
            str_bid = arr_bids(j)
            int_bid = Val(Left$(str_bid, 2))
            str_bid = bid_name(int_bid)

            If Len(str_bid) > 2 Then
                note_index = note_index + 1
                str_bid = str_bid & "*" & CStr(note_index)
                note_text = note_text & vbCrLf & "*" & CStr(note_index) & " " & Mid$(str_bid, 4)
            End If
            If lp = 0 Or j = 0 Then
                str_bidding = str_bidding & str_bid
            Else
                str_bidding = str_bidding & vbTab & str_bid
            End If
            If lp = 3 Then
                str_bidding = str_bidding & vbCrLf
            End If
            lp = (lp + 1) Mod 4
        Next j
        bidding_body = str_bidding
    End Function
End Module
