module top;
  localparam real E1 = 2.5;
  function automatic int f();
    typedef enum {E0, E1} e_t;
    return E1 + 0;
  endfunction
  localparam int KF = f();
  initial begin
    #1 $display("fn f=%0d KF=%0d", f(), KF);
    #1 $display("post %f", E1);
  end
  initial #100 $finish;
endmodule
