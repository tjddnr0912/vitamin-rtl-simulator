module sub #(parameter logic [3:0] B = 4'd0)(); initial #1 $display("A b=%b", B); endmodule
module t; typedef enum logic [3:0] {EA = 4'b1x01, EB = 4'd2} e_t;
  sub #(.B(EA)) u(); initial #100 $finish; endmodule
