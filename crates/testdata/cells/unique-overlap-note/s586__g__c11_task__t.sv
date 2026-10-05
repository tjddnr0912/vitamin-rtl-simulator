module t;
  int y;
  task automatic tk(input logic [1:0] r);
    unique casez (r)
      2'b?1: y = 1;
      2'b1?: y = 2;
    endcase
  endtask
  logic [1:0] r;
  initial begin
    r = 2'b11;
    #1;
    tk(r);
    $display("y=%0d", y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
