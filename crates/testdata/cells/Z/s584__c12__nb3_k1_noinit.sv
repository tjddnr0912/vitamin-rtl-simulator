module top;
  wire [1:0] w = 2'd3;
  logic [1:0] y;
  always_comb begin
    $display("C t=%0t w=%b", $time, w);
    y = 0;
    unique case (w) 2'd1: y = 1; 2'd2: y = 2; endcase
  end
  final $display("F y=%0d", y);
endmodule
