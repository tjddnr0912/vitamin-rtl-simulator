module t; logic clk = 0; logic [1023:0] v = 'z; logic [127:0] m [4]; logic [64:0] q65; bit [69:0] b2 = 'x; logic signed [99:0] sg;
  wand [69:0] wa = 'z; uwire [69:0] uw = 'x; longint li = 'x; logic [69:0] cmp;
  always_ff @(posedge clk) q65 <= 'x;
  initial begin m[3] <= 'x; sg = 'z; #1 clk = 1; #1 cmp = (v === {1024{1'bz}}) ? 70'd1 : 70'd0;
    $display("A ok1024=%0d v0=%h m3=%h q65=%h b2=%h sg=%h", cmp, v[69:0], m[3], q65, b2, sg); $display("B wa=%h uw=%h li=%h", wa, uw, li); end
  initial #100 $finish;
endmodule
