module top;
  logic [1:0] src = 2'd1;
  logic en;
  logic [1:0] s0, s1, s2, s3, y;
  initial en = 1'b1;
  always_comb s0 = src;
  always_comb s2 = s1;
  always_comb s1 = en ? s0 : 2'd2;
  always_comb begin s3 = s2; assert (s2 != 2'b00) else $error("A3 t=%0t s2=%b", $time, s2); end
  always_comb begin unique case (s3) 2'd1: y = 2'd1; 2'd2: y = 2'd2; endcase end
  initial #1 $display("t=%0t y=%0d s3=%0d", $time, y, s3);
  initial #5 $finish;
endmodule
