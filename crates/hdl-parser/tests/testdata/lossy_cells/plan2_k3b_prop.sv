module t; logic clk=0; logic [69:0] w = '1, v = 'z;
  always #1 clk = ~clk;
  property p(y); @(posedge clk) (v === y); endproperty
  property r(y); @(posedge clk) (w === y); endproperty
  a2: assert property (p('z)) else $display("F pz");
  a3: assert property (r('1)) else $display("F r1");
  a4: assert property (r(70'h3fffffffffffffffff)) else $display("F ctl_pass");
  a5: assert property (r(70'h1)) else $display("F ctl_fail_expected");
  initial begin #4 $display("A done"); $finish; end
endmodule
