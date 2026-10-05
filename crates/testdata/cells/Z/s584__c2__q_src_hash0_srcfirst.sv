module top;
  logic [1:0] s, y;
  initial #0 s = 2'd1;
  always_comb begin
    $display("C t=%0t s=%b", $time, s);
    unique case (s) 2'd1: y = 1; 2'd2: y = 2; endcase
  end
  initial begin #2 s = 2'd0; #1 $finish; end
  initial #100 $finish;
endmodule
