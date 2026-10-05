class C;
  function int pick(logic [1:0] r);
    unique casez (r)
      2'b?1: return 1;
      2'b1?: return 2;
      default: return 0;
    endcase
  endfunction
endclass
module top;
  C c;
  initial #100 $finish;
  initial begin
    c = new;
    #1 $display("pick=%0d", c.pick(2'b11));
  end
endmodule
