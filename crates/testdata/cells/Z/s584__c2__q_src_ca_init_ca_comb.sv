module top;
  logic [1:0] s, y;
  wire [1:0] w;
  initial begin s = 2'd1; #2 s = 2'd0; #1 $finish; end
  assign w = s;
  always_comb begin
    $display("C t=%0t w=%b", $time, w);
    unique case (w) 2'd1: y = 1; 2'd2: y = 2; endcase
  end
  initial #100 $finish;
endmodule
