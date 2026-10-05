module t;
  function automatic int f(input logic [1:0] r);
    int y;
    unique casez (r)
      2'b?1: y = 1;
      2'b1?: y = 2;
    endcase
    return y;
  endfunction
  int y; logic [1:0] r;
  initial begin
    r = 2'b11;
    #1;
    y = f(r);
    $display("y=%0d", y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
