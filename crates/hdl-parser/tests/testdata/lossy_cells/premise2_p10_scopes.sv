package p; logic [69:0] pv = 'x; logic [69:0] pw; endpackage
module m(o); output [69:0] o; reg [69:0] o = 'x; endmodule
module t; import p::*; logic [69:0] mo;
  m um(mo);
  initial begin pw = 'z; #1 $display("A pv=%h pw=%h mo=%h", p::pv, p::pw, mo); end
  initial #100 $finish;
endmodule
