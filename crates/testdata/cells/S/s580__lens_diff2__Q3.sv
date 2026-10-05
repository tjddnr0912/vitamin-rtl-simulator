`ifdef IV
 `define IN(a,b)  ((a) ==? b)
`else
 `define IN(a,b)  ((a) inside {b})
`endif
`define E1 `IN(4'd15 + 4'd1, 5'b1?000)
`define E0 `IN(4'd15 + 4'd1, 5'b0?000)
`define ES `IN(SA >>> 1, 4'sb111?)
`define EX (4'bx100 ==? 4'b1?00)
package pk;
  localparam PK1 = `E1;
  localparam PK0 = `E0;
endpackage
module sub #(parameter P = 0, parameter [31:0] K = "XXX ", parameter PD = `E1) ();
  initial #1 $display("%s%0d %0d", K, P, PD);
endmodule
module t #(parameter [3:0] TP4 = 4'd0) ();
  localparam signed [3:0] SA = -4'sd2;
  function automatic int idf(input int x); return x; endfunction
  localparam LF1 = idf(`E1);
  localparam LF0 = idf(`E0);
  localparam logic [7:0] LT1 = `E1;
  localparam LG = `IN(TP4 + 4'd1, 5'b1?000);
  logic [`E1*3:0] rb1;  logic [`E0*3+1:0] rb0;  logic [`ES*3:0] rbs;
  logic ad1 [`E1*3:0];  logic ads [`ES*3:0];
  sub #(.P(`E1), .K("OV1 ")) u1();
  sub #(.P(`ES), .K("OVS ")) u2();
  sub #(.P(`E0), .K("OV0 ")) u3();
  if (`E1) begin : g1  initial #1 $display("G1 then"); end else begin : g1e initial #1 $display("G1 else"); end
  if (`ES) begin : gs  initial #1 $display("GS then"); end else begin : gse initial #1 $display("GS else"); end
  case (`E1) 1'b1: begin : gc1 initial #1 $display("GC1 one"); end default: begin : gc1d initial #1 $display("GC1 def"); end endcase
  case (1'b1) `E0: begin : gl0 initial #1 $display("GL lab0"); end `E1: begin : gl1 initial #1 $display("GL lab1"); end default: begin : gld initial #1 $display("GL def"); end endcase
`ifdef RBT
  logic [(1 ? 3 : `EX):0] rbt; initial #1 $display("RBT %0d", $bits(rbt));
`endif
`ifdef RBB
  logic [$bits(`EX):0] rbb; initial #1 $display("RBB %0d", $bits(rbb));
`endif
`ifdef ADT
  logic adt [(1 ? 3 : `EX):0]; initial #1 $display("ADT %0d", $size(adt));
`endif
`ifdef GA
  case (1'b1) 1'b1: begin : ga initial #1 $display("GA first"); end `EX: begin : gax initial #1 $display("GA x"); end endcase
`endif
`ifdef XBEFORE
  case (1'b1) `EX: begin : gb initial #1 $display("GB x"); end 1'b1: begin : gb1 initial #1 $display("GB one"); end endcase
`endif
`ifdef TDX
  typedef logic [`EX*3:0] TX; TX tx; initial #1 $display("TDX %0d", $bits(tx));
`endif
`ifdef STX
  struct packed { logic [`EX*3:0] f; } sx; initial #1 $display("STX %0d", $bits(sx));
`endif
`ifdef REPX
  initial #1 $display("REPX %b", {(`EX + 1){1'b1}});
`endif
`ifdef FNRX
  function automatic logic [`EX*3:0] fr(); return '1; endfunction initial #1 $display("FNRX %0d", $bits(fr()));
`endif
`ifdef FBODY
  function automatic logic fb(input logic [3:0] a); return `IN(a + 4'd1, 5'b1?000); endfunction
  localparam LFB = fb(4'd15); initial #1 $display("LFB %b", LFB);
`endif
`ifdef GFOR
  for (genvar i = 0; i < `EX + 2; i++) begin : gf initial #1 $display("GF%0d x", i); end
`endif
`ifdef PSEL
  logic [7:0] v8 = 8'hA5; initial #1 $display("PSEL %b", v8[`EX*3+1:0]);
`endif
  initial begin
    #2;
    $display("PK1 %b", pk::PK1); $display("PK0 %b", pk::PK0);
    $display("LF1 %0d", LF1); $display("LF0 %0d", LF0); $display("LT1 %0d", LT1); $display("LG %b", LG);
    $display("RB1 %0d", $bits(rb1)); $display("RB0 %0d", $bits(rb0)); $display("RBS %0d", $bits(rbs));
    $display("AD1 %0d", $size(ad1)); $display("ADS %0d", $size(ads));
    #1 $finish;
  end
endmodule
