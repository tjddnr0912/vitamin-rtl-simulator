// P1: const-domain `inside`/`==?` with a pattern WIDER than the left operand's self width
module t;
  localparam [3:0] A = 4'd15;
  localparam [7:0] U8 = 8'b0000_1000;
`ifndef NO_INSIDE
  localparam L1 = (4'd15 + 4'd1) inside {5'b1?000};   // 5-bit region: 16 = 10000 -> 1
  localparam L3 = (~4'b0000) inside {5'b1111?};       // ~ at 5 bits = 11111 -> 1
  localparam L5 = (A + 4'd1) inside {5'b1?000};       // 1
  localparam L7 = (4'b1000 << 1) inside {5'b1?000};   // 1
  localparam L8 = (4'd15 + 4'd1) inside {4'b0?00};    // control, same width: 0000 -> 1
  localparam L9 = U8 inside {4'b1?000};               // oversized literal, truncated to ?000 -> 1
  logic [((4'd15 + 4'd1) inside {5'b1?000}) : 0] rb;  // range bound: [1:0] -> 2
  if ((4'd15 + 4'd1) inside {5'b1?000}) begin : g_yes
    initial #1 $display("G1 then");
  end else begin : g_no
    initial #1 $display("G1 else");
  end
`endif
  localparam L2 = (4'd15 + 4'd1) ==? 5'b1?000;        // 1
  localparam L4 = (~4'b0000) ==? 5'b1111?;            // 1
  localparam L6 = (A + 4'd1) ==? 5'b1?000;            // 1
  localparam L10 = U8 ==? 4'b1?000;                   // 1
  logic [7:0] u8;
  initial begin
    u8 = 8'b0000_1000;
`ifndef NO_INSIDE
    $display("L1=%b L3=%b L5=%b L7=%b L8=%b L9=%b bits(rb)=%0d", L1, L3, L5, L7, L8, L9, $bits(rb));
    $display("R1=%b R3=%b R5=%b R7=%b R9=%b", (4'd15 + 4'd1) inside {5'b1?000}, (~4'b0000) inside {5'b1111?},
             (A + 4'd1) inside {5'b1?000}, (4'b1000 << 1) inside {5'b1?000}, u8 inside {4'b1?000});
`endif
    $display("L2=%b L4=%b L6=%b L10=%b", L2, L4, L6, L10);
    $display("Q2=%b Q4=%b Q6=%b Q10=%b", (4'd15 + 4'd1) ==? 5'b1?000, (~4'b0000) ==? 5'b1111?, (A + 4'd1) ==? 5'b1?000, u8 ==? 4'b1?000);
    #2 $finish;
  end
endmodule
