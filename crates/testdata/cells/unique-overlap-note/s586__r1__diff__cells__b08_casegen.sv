module top #(parameter int K = 1);
  logic [1:0] r = 2'b11; int y;
  initial #100 $finish;
  case (K)
    0: begin : A initial y = 5; end
    1: begin : B
      initial begin
        #1 unique casez (r)
          2'b?1: y = 1;
          2'b1?: y = 2;
        endcase
      end
    end
  endcase
  initial #2 $display("y=%0d", y);
endmodule
