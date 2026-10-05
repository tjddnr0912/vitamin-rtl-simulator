module top;
  localparam string E1 = "AB";
  int r;
  task automatic t();
    typedef enum {E0, E1} e_t;
    r = E1 + 0;
  endtask
  initial begin
    #1 t();
    $display("tk r=%0d", r);
    #1 $display("post %s", E1);
  end
  initial #100 $finish;
endmodule
