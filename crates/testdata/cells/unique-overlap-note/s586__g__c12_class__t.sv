class C;
  function int m(logic [1:0] r);
    int y;
    unique casez (r)
      2'b?1: y = 1;
      2'b1?: y = 2;
    endcase
    return y;
  endfunction
endclass
module t;
  C c; int y; logic [1:0] r;
  initial begin
    c = new;
    r = 2'b11;
    #1;
    y = c.m(r);
    $display("y=%0d", y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
