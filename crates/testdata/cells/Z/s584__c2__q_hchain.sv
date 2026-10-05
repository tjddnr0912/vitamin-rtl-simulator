module top;
  logic [1:0] s, x, y;
  always_comb x = s;
  leafh u(.s(x), .y(y));
  initial begin s = 2'd1; #2 s = 2'd0; #1 $finish; end
  initial #100 $finish;
endmodule
module leafh(input logic [1:0] s, output logic [1:0] y);
  always_comb begin
    y = 0;
    unique case (s) 2'd1: y = 1; 2'd2: y = 2; endcase
  end
endmodule
