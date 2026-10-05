primitive inv(o, a); output o; input a; table 0:1; 1:0; endtable endprimitive
module top;
  reg k = 1'b1;
  wire n, o, q;
  reg clk;
  reg qf;
  not g(n, k);
  inv u(o, n);
  always @(posedge clk) qf <= o;
  initial begin clk = 1'b1; #1 $display("t=%0t n=%b o=%b qf=%b", $time, n, o, qf); $finish; end
endmodule
