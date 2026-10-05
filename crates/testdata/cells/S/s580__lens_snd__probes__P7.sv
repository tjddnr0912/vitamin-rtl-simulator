// P7: F1 where PRE was RIGHT (definite mismatch on the region-width bit) + S4 fill element vs a 36-bit LHS
module t;
`ifndef NO_INSIDE
  localparam L4 = (4'd15 + 4'd1) inside {5'b0?000};    // 5-bit region: 10000 vs 0?000 -> 0
  if ((4'd15 + 4'd1) inside {5'b0?000}) begin : g4y
    initial #1 $display("G4 then");
  end else begin : g4n
    initial #1 $display("G4 else");
  end
`endif
  localparam L4q = (4'd15 + 4'd1) ==? 5'b0?000;        // 0
  logic [35:0] w36;
  initial begin
    w36 = 36'h8_0000_0001;
`ifndef NO_INSIDE
    $display("L4=%b F1=%b F2=%b F3=%b", L4, w36 inside {'x}, w36 inside {'z}, w36 inside {36'h0, 'x});
`endif
    $display("L4q=%b FQ=%b", L4q, w36 ==? 'x);
    #2 $finish;
  end
endmodule
