primitive inv(o, a);
  output o; input a;
  table 0:1; 1:0; endtable
endprimitive
module top;
  logic a = 1'b0;
  wire m, o;
  int n = 0;
  inv u1(m, a);
  inv u2(o, m);
  always @(o) begin n++; $display("w t=%0t o=%b m=%b", $time, o, m); end
  initial begin
    $display("i0 t=%0t o=%b m=%b", $time, o, m);
    #0 $display("i1 t=%0t o=%b m=%b", $time, o, m);
    #0 $display("i2 t=%0t o=%b m=%b", $time, o, m);
    #1 $display("e t=%0t o=%b m=%b n=%0d", $time, o, m, n);
    a = 1'b1;
    #1 $display("e2 t=%0t o=%b m=%b n=%0d", $time, o, m, n);
    $finish;
  end
endmodule
