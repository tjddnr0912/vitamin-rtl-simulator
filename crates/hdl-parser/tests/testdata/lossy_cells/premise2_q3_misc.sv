interface pif #(parameter int W = 8); logic [W-1:0] s = 'x; wire [W-1:0] n = 'z; logic [W-1:0] s2; endinterface
module t; logic clk = 0; logic [69:0] r1, r2, r3, r4; logic [69:0] mem2 [2][3];
  pif #(.W(70)) pi();
  for (genvar g = 1; g < 3; g++) begin : gl logic [g*35-1:0] v = 'x; end
  if (1) begin : gi logic [69:0] v = 'z; end
  always @(posedge clk) begin : ab logic [69:0] v = 'x; r1 = v; end
  initial begin
    pi.s2 = 'z; mem2[1][2] = 'x;
    fork begin : fb logic [69:0] fv = 'z; r2 = fv; end join_none
    for (int i = 0; i < 1; i++) begin : lb logic [69:0] lv = 'x; r3 = lv; end
    #1 clk = 1; #1;
    $display("A gl2=%h gl1=%h gi=%h r1=%h r2=%h r3=%h", gl[2].v, gl[1].v, gi.v, r1, r2, r3);
    $display("B s=%h n=%h s2=%h m=%h", pi.s, pi.n, pi.s2, mem2[1][2]);
  end
  initial #100 $finish;
endmodule
