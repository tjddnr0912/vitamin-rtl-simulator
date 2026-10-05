module top;
  logic [1:0] a = 2'd1;
  logic [1:0] x;
  wire [1:0] y;
  always_comb x = a;
  leafh u(.s(x), .y(y));
  initial begin #2 a = 2'd0; #1 $finish; end
  initial #100 $finish;
endmodule
module leafh(input logic [1:0] s, output logic [1:0] y);
  always_comb begin
    $display("L t=%0t s=%b", $time, s);
    y = 0;
    unique case (s) 2'd1: y = 1; 2'd2: y = 2; endcase
  end
endmodule
