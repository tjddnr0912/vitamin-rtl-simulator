module top;
  wire w;
  logic x;
  wire xv;
  assign w = 1'b1;
  assign xv = x;
  always @(w) begin x = w; $display("W t=%0t", $time); end
  always @(w) $display("R t=%0t xv=%b x=%b", $time, xv, x);
  always @(xv) $display("XV t=%0t xv=%b", $time, xv);
  final $display("F xv=%b", xv);
endmodule
