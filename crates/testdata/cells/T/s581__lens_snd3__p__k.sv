module t;
  localparam int N = 2;
  localparam int W = 4;
  localparam int I = 0;
  localparam [7:0] P8 = 8'h0C;
  localparam [3:0] P4 = 4'b1100;
  function automatic logic [W-1:0] fw(input logic [W-1:0] a); return a; endfunction
  function automatic integer gi(input integer a); return a; endfunction
`ifdef C01
`define E ({2{2'b10}} ==? 4'b1?10)
`elsif C02
`define E ({N{2'b10}} ==? 4'b1?10)
`elsif C03
`define E ({N{2'b10}} !=? 4'b1?10)
`elsif C04
`define E ("AB" ==? 16'h4?42)
`elsif C05
`define E (fw(4'd12) ==? 4'b1?00)
`elsif C06
`define E (fw(4'd12) ==? 8'b0000_1?00)
`elsif C07
`define E (gi(12) ==? 8'b0000_1?00)
`elsif C08
`define E ('1 ==? 4'b1?11)
`elsif C09
`define E (P8[W-1:0] ==? 4'b1?00)
`elsif C10
`define E (P8[I +: 4] ==? 4'b1?00)
`elsif C11
`define E ((1 ? {N{2'b10}} : 4'd0) ==? 4'b1?10)
`elsif C14
`define E ({N{2'b10}} ==? 8'b0000_1?10)
`elsif C15
`define E ({P4, {N{2'b10}}} ==? 8'b1100_1?10)
`elsif C16
`define E (P8[W-1:0] ==? 8'b0000_1?00)
`elsif C18
`define E (64'h8000_0000_0000_000C ==? 4'b1?00)
`elsif C19
`define E ($unsigned(40'h10_0000_000C) ==? 4'b1?00)
`elsif C20
`define E (8'(12) ==? 16'b0000_0000_0000_1?00)
`elsif C25
`define E ((-8'sd8 / 8'sd2) ==? 8'b1111_11?0)
`elsif C26
`define E ((-8'sd8 >>> 1) ==? 8'b0111_11?0)
`elsif C27
`define E (-4'sd4 ==? 8'b1111_11?0)
`elsif C28
`define E (8'sd28 ==? 4'sbx100)
`elsif C29
`define E (4'sd12 ==? 8'b0000_1?00)
`elsif C30
`define E (4'sb1100 ==? 8'sb1111_?100)
`elsif C31
`define E ({N{2'b10}} ==? (4'b1?10))
`elsif C32
`define E ({N{2'b10}} ==? 'b1?10)
`endif
  localparam L = `E;
  if (`E) begin : gt initial #1 $display("GI=1"); end else begin : ge initial #1 $display("GI=0"); end
  initial begin $display("L=%0d", L); #2 $finish; end
endmodule
