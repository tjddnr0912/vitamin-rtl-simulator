module top;
  logic [1:0] c = 2'd1;
  logic [1:0] y;
  logic s;
  always_comb begin y = c ^ {2{s}}; $strobe("SC t=%0t y=%b", $time, y); end
  initial $monitor("M t=%0t y=%b s=%b", $time, y, s);
  initial begin s = 1'b0; #3 s = 1'b1; #3 $finish; end
  final $display("FIN y=%b", y);
endmodule
