primitive p(o, a);
  output o; input a;
  table ?:1; endtable
endprimitive
module top;
  reg a;
  wire o;
  int n = 0;
  p u(o, a);
  always @(o) begin n++; $display("w t=%0t o=%b", $time, o); end
  initial begin
    $display("i0 t=%0t o=%b", $time, o);
    #0 $display("i1 t=%0t o=%b", $time, o);
    #1 a = 1'bz;
    #1 $display("z->%b n=%0d", o, n);
    a = 1'bx;
    #1 $display("x->%b n=%0d", o, n);
    $finish;
  end
endmodule
