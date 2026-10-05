module unused_m;
  logic [1:0] r = 2'b11; int y;
  initial unique casez (r) 2'b?1: y = 1; 2'b1?: y = 2; endcase
endmodule
module top #(parameter bit G = 0);
  logic [1:0] r = 2'b11; int y;
  function automatic int f(logic [1:0] q);
    unique casez (q) 2'b?1: return 1; 2'b1?: return 2; default: return 0; endcase
  endfunction
  if (G) begin : gen_on
    initial unique casez (r) 2'b?1: y = 1; 2'b1?: y = 2; endcase
  end
  initial #100 $finish;
  initial begin
    repeat (2) begin
      #1;
      unique casez (r) 2'b?1: y = 1; 2'b1?: y = 2; endcase
    end
    $display("y=%0d", y);
  end
endmodule
