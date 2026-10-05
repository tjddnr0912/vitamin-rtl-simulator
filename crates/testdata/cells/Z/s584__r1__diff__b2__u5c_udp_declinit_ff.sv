primitive inv(o, a); output o; input a; table 0:1; 1:0; endtable endprimitive
module top;
  wire o;
  logic k = 1'b0;
  logic clk, q;
  inv u(o, k);
  always @(posedge clk) q <= o;
  initial begin clk = 1'b1; #1 $display("t=%0t q=%b o=%b", $time, q, o); $finish; end
endmodule
