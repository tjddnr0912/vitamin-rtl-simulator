primitive inv(o, a);
  output o; input a;
  table 0:1; 1:0; endtable
endprimitive
module top;
  logic a;
  wire o;
  int n = 0;
  inv u(o, a);
  always @(o) begin n++; $display("w t=%0t o=%b", $time, o); end
  initial begin
    a = 1'b0;
    $display("i0 t=%0t o=%b", $time, o);
    #0 $display("i1 t=%0t o=%b", $time, o);
    #1 $display("e t=%0t o=%b n=%0d", $time, o, n);
    a = 1'b1;
    #1 $display("e2 t=%0t o=%b n=%0d", $time, o, n);
    $finish;
  end
endmodule
