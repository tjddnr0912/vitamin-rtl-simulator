module top;
  logic a; wire v;
  assign v = (a === 1'bx);
  always @(v) $display("V t=%0t v=%b", $time, v);
  always @(posedge v) $display("PV t=%0t v=%b", $time, v);
  always @(negedge v) $display("NV t=%0t v=%b", $time, v);
  initial begin
    a = 1;
    #1 $display("t=%0t v=%b", $time, v);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
