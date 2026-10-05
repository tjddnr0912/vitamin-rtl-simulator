module m(input logic [1:0] r, output int y);
  always @(r) begin
    unique casez (r)
      2'b?1: y = 1;
      2'b1?: y = 2;
      default: y = 0;
    endcase
  end
endmodule
module t;
  logic [1:0] r; int y1, y2;
  m u1(.r(r), .y(y1));
  m u2(.r(r), .y(y2));
  initial begin
    r = 2'b11;
    #1 $display("y1=%0d y2=%0d", y1, y2);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
