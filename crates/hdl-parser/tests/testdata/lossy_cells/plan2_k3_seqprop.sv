module t; logic clk=0; logic [69:0] w = '1, v;
  always #1 clk = ~clk;
  sequence s(x); (w === x); endsequence
  property p(y); @(posedge clk) (v === y); endproperty
  property q(y); @(posedge clk) s(y); endproperty
  initial v = 'z;
  a1: assert property (@(posedge clk) s('1)) else $display("F s1");
  a2: assert property (p('z)) else $display("F pz");
  a3: assert property (q('1)) else $display("F q1");
  a4: assert property (@(posedge clk) s(70'h3fffffffffffffffff)) else $display("F ctl");
  initial begin #6 $display("A done"); $finish; end
endmodule
