module top;
  localparam real E1 = 2.5;
  int r;
  task t();
    typedef enum {E0, E1} e_t;
    r = E1 + 0;
  endtask
  initial begin
    #1 t();
    $display("tk2 r=%0d", r);
    #1 $display("post %f", E1);
  end
  initial #100 $finish;
endmodule
