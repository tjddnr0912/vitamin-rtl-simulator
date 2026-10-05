module top;
  wire w;
  logic x;
  wire xv;
  logic t;
  assign w = 1'b1;
  assign xv = x;
  always @(w) begin x = w; $display("W t=%0t", $time); end
  always @(xv) $display("XV t=%0t xv=%b", $time, xv);
  initial t = 1'b1;
  always @(t) $display("T t=%0t xv=%b x=%b", $time, xv, x);
  initial #1 begin $display("e t=%0t xv=%b", $time, xv); $finish; end
endmodule
